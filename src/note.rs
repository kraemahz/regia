use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Note {
    pub(crate) id: Uuid,
    pub(crate) created: DateTime<Utc>,
    pub(crate) content: String,
}

impl PartialOrd for Note {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl Note {
    pub fn new(content: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            created: Utc::now(),
            content: content.to_string(),
        }
    }

    pub fn fmt(&self) -> ColoredString {
        let text_color = "white";
        format!("* {}", self.content).color(text_color)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Notes {
    id: Uuid,
    group_name: String,
    notes: Vec<Note>,
}

impl Default for Notes {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            group_name: "root".to_string(),
            notes: vec![],
        }
    }
}

impl Notes {
    pub fn get_notes(&self) -> &Vec<Note> {
        &self.notes
    }

    pub fn get_note(&self, id: &Uuid) -> Option<&Note> {
        if let Ok(index) = self.notes.binary_search_by(|probe| probe.id.cmp(id)) {
            self.notes.get(index)
        } else {
            None
        }
    }

    pub fn add(&mut self, note: Note) {
        self.notes.push(note);
        self.notes
            .sort_by(|left, right| left.partial_cmp(right).unwrap());
    }

    pub fn remove(&mut self, note_id: Uuid) {
        if let Ok(index) = self.notes.binary_search_by(|probe| probe.id.cmp(&note_id)) {
            self.notes.remove(index);
        }
    }
}
