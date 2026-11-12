//! What the app knows about a school: the timetable in the app's own terms, and the absences,
//! messages and canteen menu in the shape their APIs answer with. `untis_api` holds the raw
//! timetable responses and turns them into the former.

pub mod absences;
pub mod ascii_timetable;
pub mod book2eat;
pub mod messages;
pub mod timetable;
pub mod untis_api;

use serde::{Deserialize, Deserializer};

/// Untis writes `null` where a field has no value rather than leaving it out
pub(crate) fn deserialize_null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}
