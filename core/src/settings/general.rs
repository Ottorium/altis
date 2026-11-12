use super::default_true;
use crate::models::timetable::EntityKind;
use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A screen of the app, reached through the navigation
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    Timetable,
    Messages,
    Absences,
    Book2Eat,
    Settings,
}

impl Tab {
    /// The tabs in the order the navigation lists them
    pub const ALL: [Tab; 5] = [Tab::Timetable, Tab::Messages, Tab::Absences, Tab::Book2Eat, Tab::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Timetable => "Timetable",
            Tab::Messages => "Messages",
            Tab::Absences => "Absences",
            Tab::Book2Eat => "Book2Eat",
            Tab::Settings => "Settings",
        }
    }

    /// Whether the tab can be taken out of the navigation. The settings can't: there would be
    /// no way left to bring anything back.
    pub fn can_be_hidden(self) -> bool {
        self != Tab::Settings
    }
}

/// What the timetable screen can show
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TimetableView {
    /// the student's own timetable
    #[default]
    Me,
    Class,
    Teacher,
    Room,
    /// the rooms no lesson takes up
    AvailableRooms,
    /// several timetables side by side
    Compare,
}

impl TimetableView {
    pub const ALL: [TimetableView; 6] = [
        TimetableView::Me,
        TimetableView::Class,
        TimetableView::Teacher,
        TimetableView::Room,
        TimetableView::AvailableRooms,
        TimetableView::Compare,
    ];

    /// What the view is called as the value of a form control
    pub fn key(self) -> &'static str {
        match self {
            TimetableView::Me => "Me",
            TimetableView::Class => "Class",
            TimetableView::Teacher => "Teacher",
            TimetableView::Room => "Room",
            TimetableView::AvailableRooms => "AvailableRooms",
            TimetableView::Compare => "Compare",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|view| view.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            TimetableView::AvailableRooms => "Available Rooms",
            other => other.key(),
        }
    }

    /// Which sort of entity the view shows one timetable of, `None` for the views that aren't
    /// about a single class, teacher or room
    pub fn entity_kind(self) -> Option<EntityKind> {
        match self {
            TimetableView::Class => Some(EntityKind::Class),
            TimetableView::Teacher => Some(EntityKind::Teacher),
            TimetableView::Room => Some(EntityKind::Room),
            TimetableView::Me | TimetableView::AvailableRooms | TimetableView::Compare => None,
        }
    }
}

impl From<EntityKind> for TimetableView {
    fn from(kind: EntityKind) -> Self {
        match kind {
            EntityKind::Class => TimetableView::Class,
            EntityKind::Teacher => TimetableView::Teacher,
            EntityKind::Room => TimetableView::Room,
        }
    }
}

/// How long loaded timetables are kept unless set otherwise, in minutes
const DEFAULT_CACHE_MINUTES: u32 = 60;

fn default_cache_minutes() -> u32 {
    DEFAULT_CACHE_MINUTES
}

/// How the app as a whole behaves: where it starts, what its navigation offers, how long it
/// keeps what it loaded
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GeneralSettings {
    /// the tab the app opens on
    #[serde(default)]
    pub start_tab: Tab,
    /// what the timetable screen shows when it is opened
    #[serde(default)]
    pub start_view: TimetableView,
    /// the tabs taken out of the navigation
    #[serde(default)]
    pub hidden_tabs: BTreeSet<Tab>,
    /// whether swiping the timetable changes the week
    #[serde(default = "default_true")]
    pub week_swipe: bool,
    /// how long a loaded timetable is shown before Untis is asked for it again
    #[serde(default = "default_cache_minutes")]
    pub timetable_cache_minutes: u32,
    /// whether to look for a newer release once a day and say so in a banner
    #[serde(default = "default_true")]
    pub check_for_updates: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            start_tab: Tab::default(),
            start_view: TimetableView::default(),
            hidden_tabs: BTreeSet::new(),
            week_swipe: true,
            timetable_cache_minutes: DEFAULT_CACHE_MINUTES,
            check_for_updates: true,
        }
    }
}

impl GeneralSettings {
    pub fn is_tab_shown(&self, tab: Tab) -> bool {
        !tab.can_be_hidden() || !self.hidden_tabs.contains(&tab)
    }

    /// The tabs of the navigation, in its order
    pub fn shown_tabs(&self) -> Vec<Tab> {
        Tab::ALL.into_iter().filter(|tab| self.is_tab_shown(*tab)).collect()
    }

    /// Takes a tab out of the navigation or puts it back. The app can't open on a tab that
    /// isn't there, so hiding the one it opens on moves that on to the next.
    pub fn toggle_tab(&mut self, tab: Tab) {
        if tab.can_be_hidden() && !self.hidden_tabs.remove(&tab) {
            self.hidden_tabs.insert(tab);
        }
        self.start_tab = self.initial_tab();
    }

    /// The tab the app opens on: the one that was set, unless it has been hidden since
    pub fn initial_tab(&self) -> Tab {
        match self.is_tab_shown(self.start_tab) {
            true => self.start_tab,
            false => self.shown_tabs().first().copied().unwrap_or(Tab::Settings),
        }
    }

    /// How long a loaded timetable stays valid. Never less than a minute, whatever was stored.
    pub fn cache_duration(&self) -> TimeDelta {
        TimeDelta::minutes(i64::from(self.timetable_cache_minutes.max(1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_from_before_these_existed_behave_as_the_app_always_did() {
        let settings: GeneralSettings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings, GeneralSettings::default());
        assert_eq!(settings.initial_tab(), Tab::Timetable);
        assert_eq!(settings.start_view, TimetableView::Me);
        assert_eq!(settings.shown_tabs(), Tab::ALL.to_vec());
        assert!(settings.week_swipe && settings.check_for_updates);
        assert_eq!(settings.cache_duration(), TimeDelta::hours(1));
    }

    #[test]
    fn a_hidden_tab_leaves_the_navigation_and_comes_back() {
        let mut settings = GeneralSettings::default();

        settings.toggle_tab(Tab::Book2Eat);
        assert_eq!(settings.shown_tabs(), vec![Tab::Timetable, Tab::Messages, Tab::Absences, Tab::Settings]);

        settings.toggle_tab(Tab::Book2Eat);
        assert_eq!(settings.shown_tabs(), Tab::ALL.to_vec());
    }

    #[test]
    fn the_settings_cannot_be_hidden() {
        let mut settings = GeneralSettings::default();
        settings.toggle_tab(Tab::Settings);
        assert!(settings.is_tab_shown(Tab::Settings));

        // not even by settings that were written by hand
        let settings: GeneralSettings = serde_json::from_str(r#"{"hidden_tabs": ["Settings", "Timetable"]}"#).unwrap();
        assert!(settings.is_tab_shown(Tab::Settings));
        assert!(!settings.is_tab_shown(Tab::Timetable));
    }

    #[test]
    fn the_app_opens_on_the_next_tab_if_its_start_tab_is_hidden() {
        let mut settings = GeneralSettings { start_tab: Tab::Messages, ..Default::default() };
        assert_eq!(settings.initial_tab(), Tab::Messages);

        settings.toggle_tab(Tab::Messages);
        assert_eq!((settings.initial_tab(), settings.start_tab), (Tab::Timetable, Tab::Timetable));

        for tab in [Tab::Timetable, Tab::Absences, Tab::Book2Eat] {
            settings.toggle_tab(tab);
        }
        assert_eq!(settings.initial_tab(), Tab::Settings);

        // settings written by hand can still name a hidden tab
        let by_hand: GeneralSettings = serde_json::from_str(r#"{"start_tab": "Absences", "hidden_tabs": ["Absences"]}"#).unwrap();
        assert_eq!(by_hand.initial_tab(), Tab::Timetable);
    }

    #[test]
    fn the_cache_never_lasts_less_than_a_minute() {
        let settings = GeneralSettings { timetable_cache_minutes: 0, ..Default::default() };
        assert_eq!(settings.cache_duration(), TimeDelta::minutes(1));
    }

    #[test]
    fn every_view_is_found_by_its_key() {
        for view in TimetableView::ALL {
            assert_eq!(TimetableView::from_key(view.key()), Some(view));
            // stored under the same name the picker knows it by
            assert_eq!(serde_json::to_value(view).unwrap(), view.key());
        }
        assert_eq!(TimetableView::from_key("Nothing"), None);
    }

    #[test]
    fn the_view_of_an_entity_kind_shows_that_kind() {
        for kind in EntityKind::ALL {
            assert_eq!(TimetableView::from(kind).entity_kind(), Some(kind));
        }
    }
}
