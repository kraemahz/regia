use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::ErrorKind as IOErrorKind;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use serde_yaml;
use uuid::Uuid;

mod aqua;

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
    tasks: &mut aqua::Tasks,
    config: &Config,
) -> std::io::Result<()> {
    // Go through all the ArgMatches for this function
    // due, priority, repeats, depends, content
    let priority = if let Some(priority_str) = matches.value_of("priority") {
        priority_str.parse::<u32>().unwrap()
    } else {
        0
    };

    let mut task_type = None;
    let repeat: Option<aqua::RepeatType> = if let Some(repeat_str) = matches.value_of("repeats") {
        task_type = Some(aqua::TaskType::Repeated);
        match repeat_str.to_ascii_lowercase().as_ref() {
            "daily" => Some(aqua::RepeatType::Daily),
            "weekly" => Some(aqua::RepeatType::Weekly),
            "monthly" => Some(aqua::RepeatType::Monthly),
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
            task_type = Some(aqua::TaskType::Deadline);
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
    let mut task = if task_type.is_none() {
        aqua::Task::new(String::from(content), priority)
    } else {
        aqua::Task::new_date(
            String::from(content),
            priority,
            datetime,
            task_type.unwrap(),
            repeat,
        )
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
    tasks: &mut aqua::Tasks,
    config: &Config,
) -> std::io::Result<()> {
    Ok(())
}

fn handle_task_list(tasks: &aqua::Tasks, conf: &Config) -> std::io::Result<()> {
    for task in tasks.get_tasks().iter() {
        println!("{:?}", task);
    }

    Ok(())
}

fn handle_task(matches: &ArgMatches, doc: &Config) -> std::io::Result<()> {
    let task_db_default = Path::new(".tasks.db");
    let task_db = match doc.get("contents") {
        Some(content) => match content.get("task_db") {
            Some(content) => Path::new(content),
            None => task_db_default,
        },
        None => task_db_default,
    };

    let mut tasks = match aqua::Tasks::from_disk(task_db) {
        Ok(tasks) => tasks,
        Err(err) => {
            if err.kind() == IOErrorKind::Other {
                return Err(err);
            } else {
                aqua::Tasks::new()
            }
        }
    };

    if let Some(ref matches) = matches.subcommand_matches("add") {
        handle_task_add(matches, &mut tasks, doc)?;
        tasks.to_disk(task_db)
    } else if let Some(ref matches) = matches.subcommand_matches("rm") {
        handle_task_rm(matches, &mut tasks, doc)?;
        tasks.to_disk(task_db)
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
