use super::auth::AuthHelper;
use super::webuntis_url;
use super::week::Week;
use crate::env::{header, Env, Headers};
use crate::errors::ApiError;
use crate::models::absences::*;
use crate::models::messages::*;
use crate::models::timetable::*;
use crate::models::untis_api::{UntisError, UntisPreSelected, UntisResponse};
use crate::settings::AuthSettings;
use crate::store::Store;
use chrono::NaiveDate;
use futures::future::join_all;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::collections::HashMap;
use std::marker::PhantomData;

/// How much of a response is quoted before the place it stopped making sense
const PARSE_ERROR_CONTEXT: usize = 20;

pub struct UntisClient<E: Env> {
    school_name: String,
    _env: PhantomData<E>,
}

impl<E: Env> UntisClient<E> {
    pub fn new() -> Result<Self, ApiError> {
        let school_name = Store::<E>::get_settings()?
            .ok_or(ApiError::Authentication("Settings are empty".to_string()))?
            .untis_auth
            .school_identifier;

        Ok(Self { school_name, _env: PhantomData })
    }

    pub fn is_authenticated() -> bool {
        AuthHelper::<E>::is_authenticated()
    }

    pub async fn authenticate(auth: &AuthSettings) -> Result<(), ApiError> {
        AuthHelper::<E>::authenticate(auth).await
    }

    fn url(&self, path: &str) -> String {
        webuntis_url(&self.school_name, path)
    }

    /// Performs an authorized GET request and parses the response, including Untis error checking
    async fn fetch<T: DeserializeOwned>(url: &str) -> Result<T, ApiError> {
        let response = AuthHelper::<E>::authorized_request("GET", url, Headers::new(), String::new()).await?;

        // checked first, an error response doesn't parse as the expected type
        if let Ok(error) = serde_json::from_str::<UntisError>(&response.body) {
            error.into_result()?;
        }

        parse(&response.body)
    }

    /// Performs a session-authorized classreg request and parses the response, including the
    /// error shape that API answers a refusal with
    async fn classreg<T: DeserializeOwned>(method: &str, url: &str, body: String) -> Result<T, ApiError> {
        let mut headers = Headers::from([header("Accept", "application/json")]);
        if !body.is_empty() {
            headers.extend([header("Content-Type", "application/json")]);
        }

        let response = AuthHelper::<E>::session_request(method, url, headers, body).await?;

        // checked first, an error response doesn't parse as the expected type
        if let Ok(errors) = serde_json::from_str::<ErrorResponse>(&response.body) {
            let message = errors.message();
            if !message.is_empty() {
                return Err(ApiError::Miscellaneous(message));
            }
        }

        parse(&response.body)
    }

    /// The classes of the school, and the id of the one the student is in
    async fn get_classes(&self, week: &Week) -> Result<(Vec<Class>, Option<i32>), ApiError> {
        let url = self.url(&format!(
            "api/rest/view/v1/timetable/filter?resourceType=CLASS&timetableType=STANDARD&start={}&end={}",
            week.start, week.end,
        ));

        let response: UntisResponse = Self::fetch(&url).await?;
        let classes = response.classes.unwrap_or_default().into_iter().map(Class::from).collect();
        Ok((classes, response.pre_selected.map(|class| class.id)))
    }

    /// Fetches timetable entries and fills in the days of the week that have no lessons
    async fn fetch_week(url: &str, week: &Week) -> Result<WeekTimeTable, ApiError> {
        let mut days: Vec<DayTimeTable> =
            Self::fetch::<UntisResponse>(url).await?.days.unwrap_or_default().into_iter().map(DayTimeTable::from).collect();

        for date in week.days() {
            if !days.iter().any(|day| day.date == date) {
                days.push(DayTimeTable { date, lessons: Vec::new() });
            }
        }
        days.sort_by_key(|day| day.date);

        Ok(WeekTimeTable { days })
    }

    pub async fn get_timetable(&self, week: &Week, class: Class) -> Result<WeekTimeTable, ApiError> {
        let url = self.url(&format!(
            "api/rest/view/v1/timetable/entries?start={}&end={}&format=1&resourceType=CLASS&resources={}&periodTypes=&timetableType=STANDARD&",
            week.start, week.end, class.id,
        ));

        // the entries of a class don't name the class itself
        let mut timetable = Self::fetch_week(&url, week).await?;
        for lesson in timetable.days.iter_mut().flat_map(|day| &mut day.lessons) {
            lesson.entities.push(Tracked { inner: Entity::Class(class.clone()), status: ChangeStatus::Regular });
        }

        Ok(timetable)
    }

    /// The logged in student, as far as Untis says who that is for the week
    async fn get_me(&self, week: &Week) -> Result<Option<UntisPreSelected>, ApiError> {
        let url = self.url(&format!(
            "api/rest/view/v1/timetable/filter?resourceType=STUDENT&timetableType=MY_TIMETABLE&start={}&end={}",
            week.start, week.end,
        ));

        Ok(Self::fetch::<UntisResponse>(&url).await?.pre_selected)
    }

    /// Gets the personal timetable of the logged in student, which only contains their own lessons
    pub async fn get_my_timetable(&self, week: &Week) -> Result<MyTimeTable, ApiError> {
        // Untis doesn't say who the student is for every week, the running one it always does
        let me = match self.get_me(week).await {
            Ok(Some(me)) => me,
            _ => self.get_me(&Week::current()).await?
                .ok_or(ApiError::Miscellaneous("Untis did not return a personal timetable for this account".to_string()))?,
        };

        let url = self.url(&format!(
            "api/rest/view/v1/timetable/entries?start={}&end={}&format=3&resourceType=STUDENT&resources={}&periodTypes=&timetableType=MY_TIMETABLE&layout=START_TIME",
            week.start, week.end, me.id,
        ));

        Ok(MyTimeTable {
            name: if me.display_name.is_empty() { me.short_name } else { me.display_name },
            timetable: Self::fetch_week(&url, week).await?,
        })
    }

    async fn get_timetables(&self, week: &Week, classes: Vec<Class>) -> Result<HashMap<Class, WeekTimeTable>, ApiError> {
        let requests = classes.into_iter().map(|class| async move {
            let timetable = self.get_timetable(week, class.clone()).await;
            (class, timetable)
        });

        join_all(requests)
            .await
            .into_iter()
            .map(|(class, timetable)| match timetable {
                Ok(timetable) => Ok((class, timetable)),
                Err(e) => Err(ApiError::Miscellaneous(format!("Could not get timetable for class {}: {}", class.id, e))),
            })
            .collect()
    }

    /// The timetable of every class of the school, and the id of the one the student is in
    pub async fn get_all_class_timetables(&self, week: &Week) -> Result<(HashMap<Class, WeekTimeTable>, Option<i32>), ApiError> {
        // Untis doesn't list the classes for every week, the running one it always does
        let (classes, own_class_id) = match self.get_classes(week).await {
            Ok((classes, own_class_id)) if !classes.is_empty() => (classes, own_class_id),
            _ => self.get_classes(&Week::current()).await?,
        };

        Ok((self.get_timetables(week, classes).await?, own_class_id))
    }

    /// The messages in the inbox, newest first
    pub async fn get_messages(&self) -> Result<Vec<MessagePreview>, ApiError> {
        Ok(Self::fetch::<MessageList>(&self.url("api/rest/view/v1/messages")).await?.incoming_messages)
    }

    /// WebUntis doesn't send a separate request to mark a message as read, loading it does that
    pub async fn get_message(&self, id: i32) -> Result<Message, ApiError> {
        Self::fetch(&self.url(&format!("api/rest/view/v1/messages/{id}"))).await
    }

    pub async fn get_attachment_download(&self, attachment_id: &str) -> Result<AttachmentDownload, ApiError> {
        Self::fetch(&self.url(&format!("api/rest/view/v1/messages/{attachment_id}/attachmentstorageurl"))).await
    }

    /// The id the classreg API knows the logged in student by, which is the one their personal
    /// timetable is addressed with
    async fn student_id(&self) -> Result<i32, ApiError> {
        Ok(self.get_me(&Week::current()).await?
            .ok_or(ApiError::Miscellaneous("Untis did not return a student for this account".to_string()))?
            .id)
    }

    /// The student's own absences in the given range, along with what the school lets them do
    pub async fn get_absences(&self, start: NaiveDate, end: NaiveDate) -> Result<AbsencesData, ApiError> {
        let url = self.url(&format!(
            "api/classreg/absences/students?startDate={}&endDate={}&studentId={}&excuseStatusId=-1",
            to_untis_date(start),
            to_untis_date(end),
            self.student_id().await?,
        ));

        Ok(Self::classreg::<AbsencesResponse>("GET", &url, String::new()).await?.data)
    }

    /// What the school allows and the defaults it pre-fills the report form with
    pub async fn get_absence_meta(&self) -> Result<AbsenceMeta, ApiError> {
        let url = self.url("api/classreg/absences/meta");
        Ok(Self::classreg::<AbsenceMetaResponse>("GET", &url, String::new()).await?.data)
    }

    /// Reports an absence for the logged in student
    pub async fn create_absence(&self, absence: &NewAbsence) -> Result<Absence, ApiError> {
        let mut body = absence_payload(absence);
        body["studentId"] = json!(self.student_id().await?);

        let url = self.url("api/classreg/absences/students/self");
        Ok(Self::classreg::<CreateAbsenceResponse>("POST", &url, body.to_string()).await?.data.result)
    }

    /// Changes an absence the student reported themselves. Untis wants the whole absence back,
    /// not only the fields that changed.
    pub async fn edit_absence(&self, id: i32, absence: &NewAbsence) -> Result<Absence, ApiError> {
        let mut body = absence_payload(absence);
        body["absenceId"] = json!(id);

        let url = self.url("api/classreg/absences/students/edit");
        Ok(Self::classreg::<CreateAbsenceResponse>("POST", &url, body.to_string()).await?.data.result)
    }

    /// The excuse note for a range, the PDF a student hands in signed. Untis renders it first and
    /// answers with the parameters its download URL is made of, so the file is fetched separately.
    pub async fn get_excuse_report(&self, start: NaiveDate, end: NaiveDate, excuse_group: Option<i32>) -> Result<ReportDownload, ApiError> {
        let url = self.url(&format!(
            "reports.do?name=Excuse&format=pdf&rpt_sd={}&rpt_ed={}&excuseStatusId=-1&withLateness=true&withAbsences=true&excuseGroup={}",
            to_untis_date(start),
            to_untis_date(end),
            // the schools that don't assign students a group of their own print the first one
            excuse_group.filter(|group| *group > 0).unwrap_or(1),
        ));

        let report = Self::classreg::<ReportResponse>("GET", &url, String::new()).await?.data;
        if report.error || report.report_params.is_empty() {
            return Err(ApiError::Miscellaneous("Untis could not render the excuse note".to_string()));
        }

        let format = if report.format.is_empty() { "pdf" } else { &report.format };
        Ok(ReportDownload {
            url: self.url(&format!("reports.do?{}", report.report_params)),
            file_name: format!("{}.{}", report.report_name, format),
            headers: AuthHelper::<E>::session_headers()?,
        })
    }

    /// Withdraws absences. Untis refuses the ones a teacher entered or already excused, and says
    /// so in the response rather than in a status code
    pub async fn delete_absences(&self, ids: &[i32]) -> Result<(), ApiError> {
        let url = self.url("api/classreg/absences/students");
        let body = json!({ "absenceIds": ids });
        let response: SuccessResponse = Self::classreg("DELETE", &url, body.to_string()).await?;

        match response.data.success {
            true => Ok(()),
            false => Err(ApiError::Miscellaneous("Untis did not delete the absence".to_string())),
        }
    }
}

/// The fields both reporting and editing an absence send
fn absence_payload(absence: &NewAbsence) -> serde_json::Value {
    json!({
        "startDate": to_untis_date(absence.start_date),
        "startTime": to_untis_time(absence.start_time),
        "endDate": to_untis_date(absence.end_date),
        "endTime": to_untis_time(absence.end_time),
        "text": absence.text,
        // 0 is what the web client sends for "no reason given"
        "reasonId": absence.reason_id.unwrap_or(0),
    })
}

/// Parses a response body, reporting where it stopped making sense
fn parse<T: DeserializeOwned>(body: &str) -> Result<T, ApiError> {
    serde_json::from_str(body).map_err(|e| {
        let (line, column) = (e.line(), e.column());
        let line_content = body.lines().nth(line.saturating_sub(1)).unwrap_or("");

        let context = if !line_content.is_empty() && column <= line_content.len() {
            let at = column.saturating_sub(1);
            let before = &line_content[column.saturating_sub(PARSE_ERROR_CONTEXT)..at];
            format!("{before}[-->]{}[<--] {}", &line_content[at..column], &line_content[column..])
        } else {
            line_content.to_string()
        };

        ApiError::Parsing(format!("JSON Error: {e} at line {line} col {column}.\nContext: {context}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::HttpResponse;
    use crate::settings::Settings;
    use crate::test_env::{Request, TestEnv};
    use futures::executor::block_on;

    type Client = UntisClient<TestEnv>;

    const TOKEN: &str = "eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3MDAwMDAwMDB9.c2ln";

    fn response(body: &str) -> Result<HttpResponse, String> {
        Ok(HttpResponse { headers: Headers::new(), body: body.to_string() })
    }

    /// A client for a school that logs anyone in and answers the data requests with `data`
    fn client(data: impl Fn(&Request) -> String + 'static) -> Client {
        TestEnv::reset();
        let untis_auth = AuthSettings { school_identifier: "school".to_string(), user_identifier: "max".to_string(), secret: "JBSWY3DPEHPK3PXP".to_string() };
        Store::<TestEnv>::save_settings(&Settings { untis_auth, ..Default::default() }).unwrap();

        TestEnv::respond_with(move |request| {
            if request.url.contains("jsonrpc_intern.do") {
                let cookies = ["JSESSIONID=ABC", "Tenant-Id=42", "schoolname=_c2Nob29s"];
                return Ok(HttpResponse { headers: Headers::from([("set-cookie".to_string(), cookies.map(String::from).to_vec())]), body: "{}".to_string() });
            }
            if request.url.ends_with("api/token/new") {
                return response(TOKEN);
            }
            response(&data(request))
        });
        Client::new().unwrap()
    }

    fn data_urls() -> Vec<String> {
        TestEnv::requests().into_iter().map(|r| r.url).filter(|url| !url.contains("jsonrpc_intern") && !url.ends_with("token/new")).collect()
    }

    fn week() -> Week {
        Week::from_date(NaiveDate::from_ymd_opt(2026, 9, 16).unwrap())
    }

    const STUDENT: &str = r#"{"preSelected": {"id": 555, "shortName": "MusMax", "displayName": "Max Mustermann"}}"#;
    const ONE_LESSON: &str = r##"{"days": [{"date": "2026-09-15", "gridEntries": [{
        "duration": {"start": "2026-09-15T07:50", "end": "2026-09-15T08:40"}, "type": "NORMAL_TEACHING_PERIOD", "status": "REGULAR",
        "position1": [{"current": {"type": "SUBJECT", "shortName": "M"}}]
    }]}]}"##;

    #[test]
    fn the_personal_timetable_has_every_day_of_the_week() {
        let client = client(|request| if request.url.contains("/filter?") { STUDENT } else { ONE_LESSON }.to_string());

        let mine = block_on(client.get_my_timetable(&week())).unwrap();

        assert_eq!(mine.name, "Max Mustermann");
        assert_eq!(mine.timetable.days.len(), 7);
        assert_eq!(mine.timetable.days[0].date, week().start);
        assert_eq!(mine.timetable.days[1].lessons.len(), 1);
        assert_eq!(data_urls(), vec![
            "https://school.webuntis.com/WebUntis/api/rest/view/v1/timetable/filter?resourceType=STUDENT&timetableType=MY_TIMETABLE&start=2026-09-14&end=2026-09-20",
            "https://school.webuntis.com/WebUntis/api/rest/view/v1/timetable/entries?start=2026-09-14&end=2026-09-20&format=3&resourceType=STUDENT&resources=555&periodTypes=&timetableType=MY_TIMETABLE&layout=START_TIME",
        ]);
    }

    #[test]
    fn a_class_timetable_names_the_class_on_every_lesson() {
        let client = client(|request| {
            if request.url.contains("/filter?") {
                r#"{"classes": [{"class": {"id": 101, "shortName": "4AHIT"}, "classTeacher1": null}], "preSelected": {"id": 101}}"#.to_string()
            } else {
                ONE_LESSON.to_string()
            }
        });

        let (timetables, own_class_id) = block_on(client.get_all_class_timetables(&week())).unwrap();

        assert_eq!(own_class_id, Some(101));
        let (class, timetable) = timetables.iter().next().unwrap();
        assert_eq!(class.name, "4AHIT");
        let lesson = &timetable.days[1].lessons[0];
        assert_eq!(lesson.current_names(Entity::is_class).collect::<Vec<_>>(), vec!["4AHIT".to_string()]);
        assert_eq!(
            data_urls()[1],
            "https://school.webuntis.com/WebUntis/api/rest/view/v1/timetable/entries?start=2026-09-14&end=2026-09-20&format=1&resourceType=CLASS&resources=101&periodTypes=&timetableType=STANDARD&",
        );
    }

    #[test]
    fn an_error_from_untis_is_reported_as_such() {
        let client = client(|_| r#"{"errorCode": "FORBIDDEN", "errorMessage": "No access"}"#.to_string());

        let error = block_on(client.get_messages()).unwrap_err();
        assert_eq!(error.to_string(), "Error in response from Untis: No access (FORBIDDEN)");
    }

    #[test]
    fn a_refusal_from_the_classreg_api_is_reported_in_its_own_words() {
        let client = client(|request| {
            if request.url.contains("/filter?") { STUDENT.to_string() } else { r#"{"errors": [{"title": "Not allowed", "details": null}]}"#.to_string() }
        });

        let start = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let error = block_on(client.get_absences(start, start)).unwrap_err();
        assert_eq!(error.to_string(), "Not allowed");
        assert_eq!(
            data_urls()[1],
            "https://school.webuntis.com/WebUntis/api/classreg/absences/students?startDate=20260901&endDate=20260901&studentId=555&excuseStatusId=-1",
        );
    }

    #[test]
    fn reporting_an_absence_sends_it_the_way_the_web_client_does() {
        let created = r#"{"data": {"result": {"id": 7, "startDate": 20260915, "endDate": 20260915, "startTime": 750, "endTime": 1335}}}"#;
        let client = client(move |request| match request.method.as_str() {
            "POST" => created.to_string(),
            _ if request.url.contains("/filter?") => STUDENT.to_string(),
            _ => r#"<script>{"csrfToken":"CSRF"}</script>"#.to_string(),
        });

        let day = NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
        let time = |hour, minute| chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
        let absence = NewAbsence { start_date: day, start_time: time(7, 50), end_date: day, end_time: time(13, 35), reason_id: None, text: "ill".to_string() };

        assert_eq!(block_on(client.create_absence(&absence)).unwrap().id, 7);

        let post = TestEnv::requests().into_iter().find(|request| request.method == "POST" && request.url.ends_with("students/self")).unwrap();
        let body: serde_json::Value = serde_json::from_str(&post.body).unwrap();
        assert_eq!(body, json!({"startDate": 20260915, "startTime": 750, "endDate": 20260915, "endTime": 1335, "text": "ill", "reasonId": 0, "studentId": 555}));
        assert_eq!(post.headers["X-CSRF-TOKEN"], vec!["CSRF".to_string()]);
        assert_eq!(post.headers["Content-Type"], vec!["application/json".to_string()]);
    }

    #[test]
    fn a_response_that_does_not_parse_shows_where_it_broke() {
        let error = parse::<MessageList>(r#"{"incomingMessages": [{"id": "three"}]}"#).unwrap_err();

        let message = error.to_string();
        assert!(message.starts_with("Parsing Error: JSON Error: invalid type"), "{message}");
        assert!(message.contains("[-->]"), "{message}");
    }
}
