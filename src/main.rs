use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::{self, BufRead, ErrorKind as IOErrorKind};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use colored::*;
use serde_yaml;
use uuid::Uuid;

mod db;
mod note;
mod todo;

fn expand_tilde<P: AsRef<Path>>(path_user_input: P) -> Option<PathBuf> {
    let p = path_user_input.as_ref();
    if p.starts_with("~") {
        if p == Path::new("~") {
            dirs::home_dir()
        } else {
            dirs::home_dir().map(|mut h| {
                if h == Path::new("/") {
                    // Corner case: `h` root directory;
                    // don't prepend extra `/`, just drop the tilde.
                    p.strip_prefix("~").unwrap().to_path_buf()
                } else {
                    h.push(p.strip_prefix("~/").unwrap());
                    h
                }
            })
        }
    } else {
        Some(p.to_path_buf())
    }
}

type Config = HashMap<String, HashMap<String, String>>;

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
            delete_me.push(task.clone());
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
        for task in delete_me.iter() {
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

        for task in delete_me {
            tasks.remove(task.id);
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

fn handle_task(matches: &ArgMatches, doc: &Config) -> std::io::Result<()> {
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

fn main() -> std::io::Result<()> {
    let app = App::new("regia")
        .version("0.1")
        .about("The solution to your problems")
        .author("Teague Lasser")
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("config")
                .long("config")
                .takes_value(true)
                .value_name("FILE"),
        )
        .subcommand(
            SubCommand::with_name("task")
                .setting(AppSettings::SubcommandRequired)
                .subcommand(SubCommand::with_name("ls"))
                .subcommand(
                    SubCommand::with_name("add")
                        .arg(
                            Arg::with_name("due date")
                                .short("d")
                                .long("due")
                                .takes_value(true)
                                .value_name("DATE"),
                        )
                        .arg(
                            Arg::with_name("priority")
                                .short("p")
                                .long("priority")
                                .takes_value(true)
                                .value_name("INT"),
                        )
                        .arg(
                            Arg::with_name("repeats")
                                .short("r")
                                .long("repeats")
                                .takes_value(true)
                                .value_name("PERIOD"),
                        )
                        .arg(
                            Arg::with_name("depends")
                                .short("l")
                                .long("depends")
                                .multiple(true)
                                .takes_value(true)
                                .value_name("ID"),
                        )
                        .arg(
                            Arg::with_name("content")
                                .value_name("STRING")
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("rm")
                        .arg(
                            Arg::with_name("id")
                                .long("id")
                                .takes_value(true)
                                .value_name("UUID"),
                        )
                        .arg(
                            Arg::with_name("search")
                                .required(true)
                                .value_name("STRING")
                                .min_values(1),
                        ),
                ),
        );
    let matches = app.get_matches();

    let conf_string = if matches.is_present("config") {
        let conf_arg = matches.value_of("config").unwrap();
        let conf_path = expand_tilde(conf_arg);
        read_to_string(conf_path.unwrap())?
    } else {
        let default_conf = expand_tilde("~/.config/regia/default.yml").unwrap();
        match read_to_string(default_conf) {
            Ok(contents) => contents,
            Err(_) => String::new(),
        }
    };
    let doc: Config = serde_yaml::from_str(&conf_string).unwrap();

    if let Some(ref matches) = matches.subcommand_matches("task") {
        return handle_task(matches, &doc);
    }
    Ok(())
}
