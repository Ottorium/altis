use chrono::{DateTime, Timelike, Utc};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Class {
    pub id: i32,
    pub name: String,
    pub class_teacher: Option<Teacher>,
    pub department: Department,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Department {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Teacher {
    pub id: Option<i32>, // we only have the id if it's a class teacher
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Room {
    pub name: String,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct Subject {
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct WeekTimeTable {
    pub days: Vec<DayTimeTable>,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct DayTimeTable {
    pub lessons: Vec<LessonBlock>,
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct LessonBlock {
    pub time_range: TimeRange,
    pub entities: Vec<Tracked<Entity>>,
    pub r#type: String,
    pub status: String,
    pub color_hex: String,
    pub icons: Vec<String>,
    pub texts: Vec<HashMap<String, String>>, // may include "notesAll", "statusDetail", "name", "lessonText", "substitutionText" and "userName" as well as all texts in the text response
    pub link: String,
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Debug, Serialize)]
pub enum Entity {
    Teacher(Teacher),
    Class(Class),
    Room(Room),
    Subject(Subject),
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct Tracked<T> {
    pub data: T,
    pub status: ChangeStatus,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ChangeStatus {
    Regular,
    Changed,
    Removed,
    New,
}

impl WeekTimeTable {
    pub fn to_string_pretty(
        &self,
        render_classes: bool,
        render_teacher: bool,
        render_subject: bool,
        render_room: bool,
        render_status: bool,
    ) -> String {
        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let col_width = 34;
        let time_col_width = 5;

        let mut time_points = BTreeSet::new();
        for day in &self.days {
            for lesson in &day.lessons {
                time_points.insert((lesson.time_range.start.hour(), lesson.time_range.start.minute()));
                time_points.insert((lesson.time_range.end.hour(), lesson.time_range.end.minute()));
            }
        }

        if time_points.is_empty() {
            return "No lessons scheduled.".to_string();
        }

        let sorted_points: Vec<(u32, u32)> = time_points.into_iter().collect();
        let mut slots = Vec::new();
        for i in 0..sorted_points.len() - 1 {
            slots.push((sorted_points[i], sorted_points[i + 1]));
        }

        let mut output = String::new();
        let num_days = self.days.len();
        let total_width = time_col_width + 3 + (num_days * (col_width + 3));

        output.push_str(&format!("| {:<width$} ", "Time", width = time_col_width));
        for i in 0..num_days {
            output.push_str(&format!(
                "| {:^width$} ",
                day_names.get(i).unwrap_or(&"??"),
                width = col_width
            ));
        }
        output.push_str("|\n");
        output.push_str(&format!("{:-<width$}\n", "", width = total_width));

        for ((h1, m1), (h2, m2)) in slots {
            let slot_start_mins = h1 * 60 + m1;
            let slot_end_mins = h2 * 60 + m2;
            let duration = slot_end_mins - slot_start_mins;

            let has_content = self.days.iter().any(|day| {
                day.lessons.iter().any(|l| {
                    let l_start = l.time_range.start.hour() * 60 + l.time_range.start.minute();
                    let l_end = l.time_range.end.hour() * 60 + l.time_range.end.minute();
                    l_start < slot_end_mins && l_end > slot_start_mins
                })
            });

            if !has_content { continue; }

            if duration < 35 {
                let has_significant_content = self.days.iter().any(|day| {
                    day.lessons.iter().any(|l| {
                        let l_start = l.time_range.start.hour() * 60 + l.time_range.start.minute();
                        let l_end = l.time_range.end.hour() * 60 + l.time_range.end.minute();
                        let intersects = l_start < slot_end_mins && l_end > slot_start_mins;
                        let is_strictly_spanning = l_start < slot_start_mins && l_end > slot_end_mins;
                        intersects && !is_strictly_spanning
                    })
                });
                if !has_significant_content { continue; }
            }

            let mut active_types = Vec::new();
            if render_subject { active_types.push("sub"); }
            if render_teacher { active_types.push("tea"); }
            if render_classes { active_types.push("cla"); }
            if render_room { active_types.push("roo"); }
            if render_status { active_types.push("sta"); }

            let lines_to_render = active_types.len().max(1);

            for line_idx in 0..lines_to_render {
                let time_str = match line_idx {
                    0 => format!("{:02}:{:02}", h1, m1),
                    1 => format!("{:02}:{:02}", h2, m2),
                    _ => String::new(),
                };

                output.push_str(&format!("| {:<width$} ", time_str, width = time_col_width));

                for day in &self.days {
                    let lessons_in_slot: Vec<_> = day.lessons.iter().filter(|l| {
                        let l_start = (l.time_range.start.hour(), l.time_range.start.minute());
                        let l_end = (l.time_range.end.hour(), l.time_range.end.minute());
                        l_start < (h2, m2) && l_end > (h1, m1)
                    }).collect();

                    let cell_content: Vec<String> = lessons_in_slot.iter().map(|l| {
                        let kind = active_types.get(line_idx).unwrap_or(&"");
                        match *kind {
                            "sub" => l.entities.iter().find_map(|e| if let Entity::Subject(s) = &e.data { Some(s.short_name.clone()) } else { None }).unwrap_or_default(),
                            "tea" => l.entities.iter().filter_map(|e| if let Entity::Teacher(t) = &e.data { Some(t.short_name.clone()) } else { None }).collect::<Vec<_>>().join(","),
                            "cla" => l.entities.iter().filter_map(|e| if let Entity::Class(c) = &e.data { Some(c.name.clone()) } else { None }).collect::<Vec<_>>().join(","),
                            "roo" => l.entities.iter().filter_map(|e| if let Entity::Room(r) = &e.data { Some(r.name.clone()) } else { None }).collect::<Vec<_>>().join(","),
                            "sta" => l.status.clone(),
                            _ => String::new(),
                        }
                    })
                        .filter(|s| !s.is_empty())
                        .collect();

                    let mut full_text = cell_content.join(" / ");
                    if full_text.chars().count() > col_width {
                        full_text = full_text.chars().take(col_width - 1).collect::<String>() + "…";
                    }

                    output.push_str(&format!("| {:^width$} ", full_text, width = col_width));
                }
                output.push_str("|\n");
            }
            output.push_str(&format!("{:-<width$}\n", "", width = total_width));
        }

        output
    }
}