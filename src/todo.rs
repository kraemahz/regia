use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::File;
use std::io::{
    BufReader, BufWriter, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult,
    Write,
};
use std::path::Path;
use std::string::String;
use std::vec::Vec;

use chrono::{DateTime, Utc};
use colored::*;
use rmp_serde;
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
            priority: priority,
            created: Utc::now(),
            due: None,
            content: content,
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
            priority: priority,
            created: Utc::now(),
            due: due,
            content: content,
            task_type: Some(task_type),
            repeat: repeat,
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

pub fn write_to_disk<P: AsRef<Path>>(path: P, buf: &[u8]) -> Result<(), IOError> {
    let file = File::create(path)?;
    let mut stream = BufWriter::new(file);
    stream.write_all(&buf)
}

pub fn read_from_disk<P: AsRef<Path>>(path: P) -> IOResult<Vec<u8>> {
    let file = File::open(path)?;
    let mut stream = BufReader::new(file);
    let mut data = Vec::new();
    stream.read_to_end(&mut data)?;
    Ok(data)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Tasks {
    id: Uuid,
    group_name: String,
    tasks: Vec<Task>,
}

impl Tasks {
    pub fn new() -> Tasks {
        Tasks {
            id: Uuid::new_v4(),
            group_name: "root".to_string(),
            tasks: vec![],
        }
    }

    pub fn get_tasks(&self) -> &Vec<Task> {
        &self.tasks
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

    pub fn serialize_msgpack(&self) -> Result<Vec<u8>, IOError> {
        let mut buf = Vec::new();
        match self.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
            Ok(_) => Ok(buf),
            Err(_) => Err(IOError::new(IOErrorKind::Other, "Serialization failed")),
        }
    }

    pub fn deserialize_msgpack(buf: &[u8]) -> Result<Tasks, IOError> {
        let mut de = rmp_serde::Deserializer::new(&buf[..]);
        match Tasks::deserialize(&mut de) {
            Ok(tasks) => Ok(tasks),
            Err(_) => Err(IOError::new(IOErrorKind::Other, "Deserialization failed")),
        }
    }

    pub fn from_disk<P: AsRef<Path>>(path: P) -> Result<Tasks, IOError> {
        let buf = read_from_disk(path)?;
        Tasks::deserialize_msgpack(buf.as_slice())
    }

    pub fn to_disk<P: AsRef<Path>>(&self, path: P) -> Result<(), IOError> {
        let buf = self.serialize_msgpack()?;
        write_to_disk(path, buf.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn add_and_remove_task() {
        let task = Task::new(String::from("test task"), 0);
        let mut tasks = Tasks::new();
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

        let mut tasks = Tasks::new();
        tasks.add(task.clone());
        tasks.add(subtask.clone());

        let mut file = NamedTempFile::new().unwrap();
        tasks.to_disk(file.path());

        let from_disk_tasks = Tasks::from_disk(file.path()).unwrap();
        assert_eq!(tasks, from_disk_tasks);

        for disk_task in from_disk_tasks.get_tasks().iter() {
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
