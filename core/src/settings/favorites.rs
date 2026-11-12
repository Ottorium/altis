use crate::models::timetable::EntityKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Classes, teachers and rooms marked to be found quicker, by the name the timetable picker shows
/// them under
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Favorites {
    #[serde(default)]
    pub classes: BTreeSet<String>,
    #[serde(default)]
    pub teachers: BTreeSet<String>,
    #[serde(default)]
    pub rooms: BTreeSet<String>,
}

impl Favorites {
    pub fn names(&self, kind: EntityKind) -> &BTreeSet<String> {
        match kind {
            EntityKind::Class => &self.classes,
            EntityKind::Teacher => &self.teachers,
            EntityKind::Room => &self.rooms,
        }
    }

    fn names_mut(&mut self, kind: EntityKind) -> &mut BTreeSet<String> {
        match kind {
            EntityKind::Class => &mut self.classes,
            EntityKind::Teacher => &mut self.teachers,
            EntityKind::Room => &mut self.rooms,
        }
    }

    pub fn contains(&self, kind: EntityKind, name: &str) -> bool {
        self.names(kind).contains(name)
    }

    /// Marks the name as a favourite or unmarks it
    pub fn toggle(&mut self, kind: EntityKind, name: &str) {
        let names = self.names_mut(kind);
        if !names.remove(name) {
            names.insert(name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_a_favourite_twice_removes_it() {
        let mut favorites = Favorites::default();

        favorites.toggle(EntityKind::Teacher, "MAR");
        assert!(favorites.contains(EntityKind::Teacher, "MAR"));
        // the same name of another kind is another favourite
        assert!(!favorites.contains(EntityKind::Room, "MAR"));

        favorites.toggle(EntityKind::Teacher, "MAR");
        assert!(!favorites.contains(EntityKind::Teacher, "MAR"));
    }
}
