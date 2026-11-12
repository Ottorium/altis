use crate::models::timetable::MyTimeTable;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Bookkeeping for the background notification poller, so it knows what has changed since the
/// last check and doesn't repeat itself
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NotificationState {
    /// the personal timetable as of the last poll, keyed by the week's Monday (`Week::start`),
    /// diffed against on the next poll
    #[serde(default)]
    pub last_my_timetable: BTreeMap<String, MyTimeTable>,
    /// ids of messages that have already been seen, so only new ones trigger a notification
    #[serde(default)]
    pub known_message_ids: BTreeSet<i32>,
    /// ids of absences that have already been seen, the same way
    #[serde(default)]
    pub known_absence_ids: BTreeSet<i32>,
    /// (lesson identity, lead time in minutes) pairs an exam reminder was already sent for
    #[serde(default)]
    pub sent_exam_reminders: BTreeSet<(String, i64)>,
    /// (day, title, body) of every timetable change notification sent, so the very same one is
    /// never sent twice, whatever the diff makes of the next poll
    #[serde(default)]
    pub sent_change_notifications: BTreeSet<(NaiveDate, String, String)>,
}

/// How many seen ids are kept before the ones Untis no longer lists are forgotten
const KNOWN_IDS_LIMIT: usize = 500;

/// Records the ids as seen and returns the ones that weren't before.
///
/// The very first time there is nothing to compare against, so everything is only recorded:
/// starting the app for the first time must not notify about the whole inbox or every absence of
/// the school year.
pub(super) fn record_seen(known: &mut BTreeSet<i32>, current: &[i32]) -> BTreeSet<i32> {
    let first_run = known.is_empty();
    let unseen: BTreeSet<i32> = current.iter().copied().filter(|id| known.insert(*id)).collect();

    // keep the known ids bounded rather than growing forever
    if known.len() > KNOWN_IDS_LIMIT {
        known.retain(|id| current.contains(id));
    }

    match first_run {
        true => BTreeSet::new(),
        false => unseen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_run_only_records() {
        let mut known = BTreeSet::new();

        assert!(record_seen(&mut known, &[1, 2, 3]).is_empty());
        assert_eq!(known, BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn afterwards_only_what_is_new_comes_back() {
        let mut known = BTreeSet::from([1, 2]);

        assert_eq!(record_seen(&mut known, &[3, 2, 1]), BTreeSet::from([3]));
        assert!(record_seen(&mut known, &[3, 2, 1]).is_empty());
    }

    #[test]
    fn forgets_what_untis_no_longer_lists_once_there_is_too_much_of_it() {
        let mut known: BTreeSet<i32> = (0..KNOWN_IDS_LIMIT as i32).collect();

        assert_eq!(record_seen(&mut known, &[1000, 1001]), BTreeSet::from([1000, 1001]));
        assert_eq!(known, BTreeSet::from([1000, 1001]));
    }
}
