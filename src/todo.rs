use std::cmp::Ordering;
use std::collections::HashSet;
use std::string::String;
use std::vec::Vec;

use chrono::{DateTime, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum TaskType {
    Deadline,
    Repeated,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum RepeatType {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub(crate) id: Uuid,
    pub(crate) priority: u32,
    pub(crate) created: DateTime<Utc>,
    pub(crate) due: Option<DateTime<Utc>>,
    pub(crate) content: String,
    pub(crate) task_type: Option<TaskType>,
    pub(crate) repeat: Option<RepeatType>,
    pub(crate) depends: HashSet<Uuid>,
}

impl Task {
    pub fn new(content: String, priority: u32) -> Self {
        Task {
            id: Uuid::new_v4(),
            priority,
            created: Utc::now(),
            due: None,
            content,
            task_type: None,
            repeat: None,
            depends: HashSet::new(),
        }
    }

    pub fn new_date(
        content: String,
        priority: u32,
        due: Option<DateTime<Utc>>,
        task_type: TaskType,
        repeat: Option<RepeatType>,
    ) -> Self {
        Task {
            id: Uuid::new_v4(),
            priority,
            created: Utc::now(),
            due,
            content,
            task_type: Some(task_type),
            repeat,
            depends: HashSet::new(),
        }
    }

    pub fn fmt(&self, priority_map: &[(u32, &str)]) -> ColoredString {
        let mut text_color = "white";
        for (pri, col) in priority_map {
            if self.priority < *pri {
                text_color = col;
                break;
            }
        }
        format!("* {}", self.content).color(text_color)
    }

    pub fn add_dependency(&mut self, task_id: &Uuid) {
        self.depends.insert(task_id.clone());
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Task) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Task) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Tasks {
    id: Uuid,
    group_name: String,
    tasks: Vec<Task>,
}

impl Default for Tasks {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            group_name: "root".to_string(),
            tasks: vec![],
        }
    }
}

impl Tasks {
    pub fn get_tasks(&self) -> &Vec<Task> {
        &self.tasks
    }

    pub fn get_task(&self, id: &Uuid) -> Option<&Task> {
        if let Ok(index) = self.tasks.binary_search_by(|probe| probe.id.cmp(&id)) {
            self.tasks.get(index)
        } else {
            None
        }
    }

    pub fn add(&mut self, task: Task) {
        self.tasks.push(task);
        self.tasks
            .sort_by(|left, right| left.partial_cmp(right).unwrap());
    }

    pub fn remove(&mut self, task_id: Uuid) {
        if let Ok(index) = self.tasks.binary_search_by(|probe| probe.id.cmp(&task_id)) {
            self.tasks.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::NamedTempFile;

    #[test]
    fn add_and_remove_task() {
        let task = Task::new(String::from("test task"), 0);
        let mut tasks = Tasks::default();
        tasks.add(task.clone());
        assert_eq!(&vec![task.clone()], tasks.get_tasks());
        tasks.remove(task.id);
        assert_eq!(&Vec::<Task>::new(), tasks.get_tasks());
    }

    #[test]
    fn to_from_disk() {
        let mut task = Task::new(String::from("test task"), 0);
        let subtask = Task::new(String::from("subtask"), 0);

        task.add_dependency(&subtask.id);

        let mut db = Database::default();
        {
            let tasks = &mut db.tasks;
            tasks.add(task.clone());
            tasks.add(subtask.clone());
        }

        let mut file = NamedTempFile::new().unwrap();
        db.to_disk(file.path());

        let from_disk_db = Database::from_disk(file.path()).unwrap();
        assert_eq!(db.tasks, from_disk_db.tasks);

        for disk_task in from_disk_db.tasks.get_tasks().iter() {
            match disk_task.id {
                id if id == task.id => {
                    assert_eq!(&task, disk_task);
                }
                id if id == subtask.id => assert_eq!(&subtask, disk_task),
                _ => {
                    unreachable!();
                }
            }
        }
    }
}
