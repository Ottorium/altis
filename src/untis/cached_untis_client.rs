use crate::data_models::clean_models::untis::{Entity, WeekTimeTable};
use crate::errors::ApiError;
use crate::persistence_manager::{PersistenceManager, TimeTableCache, TimeTables};
use crate::untis::teacher_table_generator::gen_all_timetables;
use crate::untis::untis_client::UntisClient;
use crate::untis::untis_week::Week;
use chrono::Local;
use std::collections::HashMap;

pub struct CachedUntisClient {
    untis_client: UntisClient,
}

impl CachedUntisClient {
    pub fn new() -> Result<Self, ApiError> {
        Ok(Self {
            untis_client: UntisClient::new()?,
        })
    }

    pub async fn get_class_timetables(&self, week: Week) -> Result<TimeTables, ApiError> {
        let cache = PersistenceManager::get_timetables()?;
        let mut cache_table = HashMap::new();
        if let Some(mut c) = cache {
            if let Some((e, t)) = c.tables.get(&week) {
                if e.is_none() {
                    return Ok(t.clone());
                }
                if e.unwrap() > Local::now().naive_local() {
                    return Ok(t.clone());
                } else {
                    c.tables.remove(&week);
                }
            }
            cache_table = c.tables.clone();
        }

        let tt = self
            .untis_client
            .get_all_class_timetables(week.clone())
            .await?;
        cache_table.insert(week, (None, tt.clone()));
        PersistenceManager::save_timetables(&TimeTableCache {
            tables: cache_table,
        })?;
        Ok(tt)
    }

    pub async fn get_all_timetables(
        &self,
        week: Week,
    ) -> Result<(HashMap<Entity, WeekTimeTable>, Option<i32>), ApiError> {
        let r = self.get_class_timetables(week).await?;
        gen_all_timetables(r.0, r.1)
    }

    pub fn clear_cache() -> Result<(), ApiError> {
        PersistenceManager::save_timetables(&TimeTableCache {
            tables: HashMap::new(),
        })
        .map_err(ApiError::from)
    }
}
