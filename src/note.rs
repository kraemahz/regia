use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Note {}

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
