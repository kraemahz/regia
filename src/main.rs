use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use clap::{App, AppSettings, Arg, SubCommand};
use serde_yaml;

mod conf;
mod db;
mod note;
mod notetaker;
mod taskmaster;
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
            SubCommand::with_name("note")
                .setting(AppSettings::SubcommandRequired)
                .subcommand(SubCommand::with_name("ls"))
                .subcommand(
                    SubCommand::with_name("add").arg(
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
    let doc: conf::Config = serde_yaml::from_str(&conf_string).unwrap();

    if let Some(ref matches) = matches.subcommand_matches("task") {
        taskmaster::handle_it(matches, &doc)
    } else if let Some(ref matches) = matches.subcommand_matches("note") {
        notetaker::handle_it(matches, &doc)
    } else {
        unreachable!();
    }
}
