use std::io::{self, BufRead, ErrorKind as IOErrorKind};
use std::path::Path;

use chrono::{DateTime, Utc};
use clap::ArgMatches;
use colored::*;
use uuid::Uuid;

use crate::conf::Config;
use crate::db;
use crate::todo;

fn handle_task_add(
    matches: &ArgMatches,
    tasks: &mut todo::Tasks,
    _doc: &Config,
) -> std::io::Result<()> {
    // Go through all the ArgMatches for this function
    // due, priority, repeats, depends, content
    let priority = if let Some(priority_str) = matches.value_of("priority") {
        priority_str.parse::<u32>().unwrap()
    } else {
        0
    };

    let mut task_type = None;
    let repeat: Option<todo::RepeatType> = if let Some(repeat_str) = matches.value_of("repeats") {
        task_type = Some(todo::TaskType::Repeated);
        match repeat_str.to_ascii_lowercase().as_ref() {
            "daily" => Some(todo::RepeatType::Daily),
            "weekly" => Some(todo::RepeatType::Weekly),
            "monthly" => Some(todo::RepeatType::Monthly),
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "bad repeats string",
                ))
            }
        }
    } else {
        None
    };

    let content = matches.value_of("content").unwrap();

    let datetime: Option<DateTime<Utc>> = if let Some(due_date) = matches.value_of("due date") {
        if task_type.is_none() {
            task_type = Some(todo::TaskType::Deadline);
        }
        match DateTime::parse_from_rfc2822(due_date) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "bad datetime string",
                ));
            }
        }
    } else {
        None
    };

    // Build the input from the matches
    let mut task = if let Some(task_type) = task_type {
        todo::Task::new_date(String::from(content), priority, datetime, task_type, repeat)
    } else {
        todo::Task::new(String::from(content), priority)
    };

    if let Some(deps) = matches.values_of("depends") {
        for dep in deps {
            let uuid = match Uuid::parse_str(dep) {
                Ok(ok) => ok,
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("bad depends uuid: {}", dep),
                    ));
                }
            };
            task.add_dependency(&uuid);
        }
    }

    // Add it to Tasks
    tasks.add(task);

    // Handle any pruning of data

    Ok(())
}

fn handle_task_rm(
    matches: &ArgMatches,
    tasks: &mut todo::Tasks,
    _doc: &Config,
) -> std::io::Result<()> {
    let search = matches.value_of("search").unwrap();
    let mut delete_me = Vec::new();

    for task in tasks.get_tasks() {
        if task.content.contains(search) {
            delete_me.push(task.id.clone());
        }
    }

    let delete_len = delete_me.len();

    if delete_len > 0 {
        println!(
            "Found {} task{} that match{}:",
            format!("{}", delete_len).magenta(),
            if delete_len > 1 { "s" } else { "" },
            if delete_len > 1 { "" } else { "es" }
        );
        for id in delete_me.iter() {
            let task = tasks.get_task(id).unwrap();
            println!("{}", task.fmt(&[]));
        }
        println!("{} [{}/{}]", "Complete?".magenta(), "y".bold(), "N".bold());
        let stdin = io::stdin();
        let mut stdin_iter = stdin.lock().lines();
        loop {
            let next_line = stdin_iter.next().unwrap().unwrap();
            if next_line.to_lowercase() == "y" {
                break;
            } else if next_line == "" || next_line.to_lowercase() == "n" {
                return Ok(());
            } else {
                println!("Didn't understand {} please type y or n", next_line);
            }
        }

        for id in delete_me {
            tasks.remove(id);
        }
    }

    Ok(())
}

fn handle_task_list(tasks: &todo::Tasks, _doc: &Config) -> std::io::Result<()> {
    let mut tasks_list = tasks.get_tasks().clone();
    tasks_list.sort_by_key(|k| k.created);
    for task in tasks_list.iter().rev() {
        println!("{}", task.fmt(&[]));
    }
    Ok(())
}

pub fn handle_it(matches: &ArgMatches, doc: &Config) -> std::io::Result<()> {
    let db_default = Path::new(".regia.db");
    let db_path = match doc.get("contents") {
        Some(content) => match content.get("regia_db") {
            Some(content) => Path::new(content),
            None => db_default,
        },
        None => db_default,
    };

    let db = match db::Database::from_disk(db_path) {
        Ok(db) => db,
        Err(err) => {
            if err.kind() == IOErrorKind::Other {
                return Err(err);
            } else {
                db::Database::default()
            }
        }
    };

    let mut tasks = db.tasks;

    if let Some(ref matches) = matches.subcommand_matches("add") {
        handle_task_add(matches, &mut tasks, doc)?;
        let new_db = db::Database {
            tasks,
            notes: db.notes,
        };
        new_db.to_disk(db_path)
    } else if let Some(ref matches) = matches.subcommand_matches("rm") {
        handle_task_rm(matches, &mut tasks, doc)?;
        let new_db = db::Database {
            tasks,
            notes: db.notes,
        };
        new_db.to_disk(db_path)
    } else {
        handle_task_list(&tasks, doc)
    }
}
