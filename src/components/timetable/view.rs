use altis_core::models::timetable::EntityKind;

/// What the timetable screen can show
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimetableView {
    /// the student's own timetable
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_view_is_found_by_its_key() {
        for view in TimetableView::ALL {
            assert_eq!(TimetableView::from_key(view.key()), Some(view));
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
