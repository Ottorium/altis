use crate::persistence_manager::{MyTimeTableCache, PersistenceManager, TimeTableCache, TimeTables};
use crate::untis::UntisClient;
use crate::untis::teacher_table_generator::gen_all_timetables;
use altis_core::data_models::clean_models::untis::{Entity, MyTimeTable, WeekTimeTable};
use altis_core::errors::ApiError;
use altis_core::untis::untis_week::Week;
use chrono::{Local, NaiveDateTime, TimeDelta};
use std::collections::HashMap;

/// How long loaded timetables are valid before they are requested again
pub const CACHE_DURATION: TimeDelta = TimeDelta::hours(1);

pub type AllTimeTables = (HashMap<Entity, WeekTimeTable>, Option<i32>);

pub struct CachedUntisClient {
    untis_client: UntisClient,
}

impl CachedUntisClient {
    pub fn new() -> Result<Self, ApiError> {
        Ok(Self {
            untis_client: UntisClient::new()?,
        })
    }

    /// Class timetables of the given weeks that are cached and not expired, along with their expiry.
    /// Never makes a request
    fn cached_class_timetables(weeks: &[Week]) -> Vec<(Week, NaiveDateTime, TimeTables)> {
        if weeks.is_empty() {
            return vec![];
        }
        let Some(cache) = PersistenceManager::get_timetables().ok().flatten() else { return vec![] };
        let now = Local::now().naive_local();
        cache.tables.into_iter()
            .filter_map(|(week, (expiry, tables))| match expiry {
                Some(expiry) if expiry > now && weeks.contains(&week) => Some((week, expiry, tables)),
                _ => None,
            })
            .collect()
    }

    /// All timetables of the given weeks that are cached and not expired, along with their expiry.
    /// Never makes a request
    pub fn cached_all_timetables(weeks: &[Week]) -> Vec<(Week, Result<(NaiveDateTime, AllTimeTables), ApiError>)> {
        Self::cached_class_timetables(weeks).into_iter()
            .map(|(week, expiry, t)| (week, gen_all_timetables(t.0, t.1).map(|all| (expiry, all))))
            .collect()
    }

    /// Personal timetables of the given weeks that are cached and not expired, along with their expiry.
    /// Never makes a request
    pub fn cached_my_timetables(weeks: &[Week]) -> Vec<(Week, NaiveDateTime, MyTimeTable)> {
        if weeks.is_empty() {
            return vec![];
        }
        let now = Local::now().naive_local();
        PersistenceManager::get_my_timetables().ok().flatten()
            .map(|c| c.tables).unwrap_or_default()
            .into_iter()
            .filter(|(week, (expiry, _))| *expiry > now && weeks.contains(week))
            .map(|(week, (expiry, t))| (week, expiry, t))
            .collect()
    }

    pub async fn get_class_timetables(&self, week: Week) -> Result<(NaiveDateTime, TimeTables), ApiError> {
        if let Some((_, expiry, tt)) = Self::cached_class_timetables(std::slice::from_ref(&week)).pop() {
            return Ok((expiry, tt));
        }

        let tt = self
            .untis_client
            .get_all_class_timetables(week.clone())
            .await?;
        let now = Local::now().naive_local();
        let expiry = now + CACHE_DURATION;

        // re-read the cache, other weeks might have been cached while we were loading
        let mut cache_table = PersistenceManager::get_timetables().ok().flatten().map(|c| c.tables).unwrap_or_default();
        cache_table.retain(|_, (e, _)| e.is_some_and(|e| e > now));
        cache_table.insert(week, (Some(expiry), tt.clone()));
        PersistenceManager::save_timetables(&TimeTableCache {
            tables: cache_table,
        })?;
        Ok((expiry, tt))
    }

    pub async fn get_all_timetables(&self, week: Week) -> Result<(NaiveDateTime, AllTimeTables), ApiError> {
        let (expiry, (classes, pre_selected)) = self.get_class_timetables(week).await?;
        Ok((expiry, gen_all_timetables(classes, pre_selected)?))
    }

    pub async fn get_my_timetable(&self, week: Week) -> Result<(NaiveDateTime, MyTimeTable), ApiError> {
        if let Some((_, expiry, t)) = Self::cached_my_timetables(std::slice::from_ref(&week)).pop() {
            return Ok((expiry, t));
        }

        let tt = self.untis_client.get_my_timetable(week.clone()).await?;
        let now = Local::now().naive_local();
        let expiry = now + CACHE_DURATION;

        // the personal timetable is only a small convenience cache, failing to save it shouldn't fail the request
        let mut cache = PersistenceManager::get_my_timetables().ok().flatten().unwrap_or_default();
        cache.tables.retain(|_, (e, _)| *e > now);
        cache.tables.insert(week, (expiry, tt.clone()));
        let _ = PersistenceManager::save_my_timetables(&cache);
        Ok((expiry, tt))
    }

    pub fn clear_cache() -> Result<(), ApiError> {
        PersistenceManager::save_my_timetables(&MyTimeTableCache::default())?;
        PersistenceManager::save_timetables(&TimeTableCache {
            tables: HashMap::new(),
        })
            .map_err(ApiError::from)
    }
}
