use crate::storage::{self, ClassTimeTableCache, ClassTimeTables, MyTimeTableCache};
use crate::untis::UntisClient;
use crate::untis::entity_timetables::entity_timetables;
use altis_core::errors::ApiError;
use altis_core::models::timetable::{Entity, MyTimeTable, WeekTimeTable};
use altis_core::untis::week::Week;
use chrono::{Local, NaiveDateTime, TimeDelta};
use std::collections::HashMap;

/// How long loaded timetables are valid before they are requested again
const CACHE_DURATION: TimeDelta = TimeDelta::hours(1);

/// The timetable of every class, teacher and room of a week, and the id of the class the student
/// is in
pub type AllTimeTables = (HashMap<Entity, WeekTimeTable>, Option<i32>);

/// Something that was loaded, along with the time it expires
pub type Expiring<T> = (NaiveDateTime, T);

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

fn with_teachers_and_rooms((classes, own_class_id): ClassTimeTables) -> AllTimeTables {
    (entity_timetables(classes), own_class_id)
}

/// The Untis client with the timetables it loaded kept in localStorage for a while, so paging
/// back and forth between weeks doesn't ask Untis for the same week again
pub struct CachedUntisClient {
    untis_client: UntisClient,
}

impl CachedUntisClient {
    pub fn new() -> Result<Self, ApiError> {
        Ok(Self { untis_client: UntisClient::new()? })
    }

    /// Class timetables of the given weeks that are cached and not expired. Never makes a request
    fn cached_class_timetables(weeks: &[Week]) -> Vec<(Week, Expiring<ClassTimeTables>)> {
        if weeks.is_empty() {
            return vec![];
        }
        let Some(cache) = storage::class_timetables().ok().flatten() else { return vec![] };
        let now = now();

        cache
            .tables
            .into_iter()
            .filter_map(|(week, (expiry, timetables))| match expiry {
                Some(expiry) if expiry > now && weeks.contains(&week) => Some((week, (expiry, timetables))),
                _ => None,
            })
            .collect()
    }

    /// All timetables of the given weeks that are cached and not expired. Never makes a request
    pub fn cached_all_timetables(weeks: &[Week]) -> Vec<(Week, Expiring<AllTimeTables>)> {
        Self::cached_class_timetables(weeks)
            .into_iter()
            .map(|(week, (expiry, timetables))| (week, (expiry, with_teachers_and_rooms(timetables))))
            .collect()
    }

    /// Personal timetables of the given weeks that are cached and not expired. Never makes a request
    pub fn cached_my_timetables(weeks: &[Week]) -> Vec<(Week, Expiring<MyTimeTable>)> {
        if weeks.is_empty() {
            return vec![];
        }
        let now = now();

        storage::my_timetables()
            .ok()
            .flatten()
            .unwrap_or_default()
            .tables
            .into_iter()
            .filter(|(week, (expiry, _))| *expiry > now && weeks.contains(week))
            .collect()
    }

    async fn get_class_timetables(&self, week: Week) -> Result<Expiring<ClassTimeTables>, ApiError> {
        if let Some((_, cached)) = Self::cached_class_timetables(std::slice::from_ref(&week)).pop() {
            return Ok(cached);
        }

        let timetables = self.untis_client.get_all_class_timetables(&week).await?;
        let now = now();
        let expiry = now + CACHE_DURATION;

        // re-read the cache, other weeks might have been cached while we were loading
        let mut cache = storage::class_timetables().ok().flatten().unwrap_or_default();
        cache.tables.retain(|_, (expiry, _)| expiry.is_some_and(|expiry| expiry > now));
        cache.tables.insert(week, (Some(expiry), timetables.clone()));
        storage::save_class_timetables(&cache)?;

        Ok((expiry, timetables))
    }

    pub async fn get_all_timetables(&self, week: Week) -> Result<Expiring<AllTimeTables>, ApiError> {
        let (expiry, timetables) = self.get_class_timetables(week).await?;
        Ok((expiry, with_teachers_and_rooms(timetables)))
    }

    pub async fn get_my_timetable(&self, week: Week) -> Result<Expiring<MyTimeTable>, ApiError> {
        if let Some((_, cached)) = Self::cached_my_timetables(std::slice::from_ref(&week)).pop() {
            return Ok(cached);
        }

        let timetable = self.untis_client.get_my_timetable(&week).await?;
        let now = now();
        let expiry = now + CACHE_DURATION;

        // the personal timetable is only a small convenience cache, failing to save it shouldn't fail the request
        let mut cache = storage::my_timetables().ok().flatten().unwrap_or_default();
        cache.tables.retain(|_, (expiry, _)| *expiry > now);
        cache.tables.insert(week, (expiry, timetable.clone()));
        let _ = storage::save_my_timetables(&cache);

        Ok((expiry, timetable))
    }

    pub fn clear_cache() -> Result<(), ApiError> {
        storage::save_my_timetables(&MyTimeTableCache::default())?;
        storage::save_class_timetables(&ClassTimeTableCache::default())?;
        Ok(())
    }
}
