use std::io::{self, BufRead, ErrorKind as IOErrorKind};
use std::path::Path;

use clap::ArgMatches;
use colored::*;

use crate::conf::Config;
use crate::db;
use crate::note;

fn handle_note_add(
    matches: &ArgMatches,
    notes: &mut note::Notes,
    _doc: &Config,
) -> std::io::Result<()> {
    let content = matches.value_of("content").unwrap();
    let note = note::Note::new(content);
    notes.add(note);
    Ok(())
}

fn handle_note_rm(
    matches: &ArgMatches,
    notes: &mut note::Notes,
    _doc: &Config,
) -> std::io::Result<()> {
    let search = matches.value_of("search").unwrap();
    let mut delete_me = Vec::new();

    for note in notes.get_notes() {
        if note.content.contains(search) {
            delete_me.push(note.id.clone());
        }
    }

    let delete_len = delete_me.len();

    if delete_len > 0 {
        println!(
            "Found {} note{} that match{}:",
            format!("{}", delete_len).magenta(),
            if delete_len > 1 { "s" } else { "" },
            if delete_len > 1 { "" } else { "es" }
        );
        for id in delete_me.iter() {
            let note = notes.get_note(id).unwrap();
            println!("{}", note.fmt());
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
            notes.remove(id);
        }
    }

    Ok(())
}

fn handle_note_list(notes: &note::Notes, _doc: &Config) -> std::io::Result<()> {
    let mut notes_list = notes.get_notes().clone();
    notes_list.sort_by_key(|k| k.created);
    for note in notes_list.iter().rev() {
        println!("{}", note.fmt());
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

    let mut notes = db.notes;

    if let Some(ref matches) = matches.subcommand_matches("add") {
        handle_note_add(matches, &mut notes, doc)?;
        let new_db = db::Database {
            tasks: db.tasks,
            notes,
        };
        new_db.to_disk(db_path)
    } else if let Some(ref matches) = matches.subcommand_matches("rm") {
        handle_note_rm(matches, &mut notes, doc)?;
        let new_db = db::Database {
            tasks: db.tasks,
            notes,
        };
        new_db.to_disk(db_path)
    } else {
        handle_note_list(&notes, doc)
    }
}
