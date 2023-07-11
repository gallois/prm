mod cli;

use clap::builder::ArgAction;
use clap::{Args, Parser, Subcommand};
use ics::properties::{Comment, DtStart, Due, RRule, Status, Summary};
use ics::{escape_text, Event as IcsEvent, ICalendar, ToDo};
use prm::db_interface::DbOperations;
use prm::entities::activity::Activity;
use prm::entities::event::{Event, EventType};
use prm::entities::note::Note;
use prm::entities::person::Person;
use prm::entities::reminder::Reminder;
use prm::entities::Entities;
use rusqlite::Connection;
use uuid::Uuid;

use std::process::exit;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {},
    Add(AddArgs),
    Show(ShowArgs),
    Edit(EditArgs),
    Remove(RemoveArgs),
    List(ListArgs),
    Ics(IcsArgs),
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
struct AddArgs {
    #[command(subcommand)]
    entity: AddEntity,
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ShowArgs {
    #[command(subcommand)]
    entity: ShowEntity,
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
struct EditArgs {
    #[command(subcommand)]
    entity: EditEntity,
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ListArgs {
    #[command(subcommand)]
    entity: ListEntity,
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
struct RemoveArgs {
    #[command(subcommand)]
    entity: RemoveEntity,
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
struct IcsArgs {
    #[arg(short, long)]
    birthdays: bool,
    #[arg(short, long)]
    reminders: bool,
    #[arg(short, long)]
    all: bool,
}

#[derive(Subcommand)]
enum AddEntity {
    Person {
        name: Option<String>,
        #[arg(short, long)]
        birthday: Option<String>,
        #[arg(short, long, action=ArgAction::Append)]
        contact_info: Option<Vec<String>>,
    },
    Activity {
        name: Option<String>,
        #[arg(short, long)]
        activity_type: Option<String>,
        #[arg(short, long)]
        date: Option<String>,
        #[arg(short, long)]
        content: Option<String>,
        #[arg(short, long)]
        people: Vec<String>,
    },
    Reminder {
        name: Option<String>,
        #[arg(short, long)]
        date: Option<String>,
        #[arg(short, long)]
        recurring: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(short, long)]
        people: Vec<String>,
    },
    Notes {
        content: Option<String>,
        #[arg(short, long)]
        people: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ShowEntity {
    Person {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        birthday: Option<String>,
    },
    Activity {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        person: Option<String>,
    },
    Reminder {
        #[arg(short, long)]
        name: String,
        // TODO Filters
    },
    Notes {
        #[arg(short, long)]
        person: String,
        // TODO Filters
    },
}

#[derive(Subcommand)]
enum EditEntity {
    // TODO implement the remaining properties
    Person {
        #[arg(short, long)]
        id: u64,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        birthday: Option<String>,
        #[arg(short, long)]
        contact_info: Option<String>,
    },
    Activity {
        #[arg(short, long)]
        id: u64,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        activity_type: Option<String>,
        #[arg(short, long)]
        date: Option<String>,
        #[arg(short, long)]
        content: Option<String>,
    },
    Reminder {
        #[arg(short, long)]
        id: u64,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        date: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(short, long)]
        recurring: Option<String>,
    },
    Note {
        #[arg(short, long)]
        id: u64,
        #[arg(short, long)]
        date: Option<String>,
        #[arg(short, long)]
        content: Option<String>,
    },
}

#[derive(Subcommand)]
enum ListEntity {
    // TODO add some filtering
    People,
    Activities,
    Reminders {
        #[arg(short, long, action = ArgAction::SetTrue)]
        include_past: bool,
    },
    Notes,
    Events {
        #[arg(short, long, default_value = "90")]
        days: u64,
    },
}

// TODO add other means of removing
#[derive(Subcommand)]
enum RemoveEntity {
    Person {
        #[arg(short, long)]
        name: String,
    },
    Activity {
        #[arg(short, long)]
        name: String,
    },
    Reminder {
        #[arg(short, long)]
        name: String,
    },
    Note {
        #[arg(short, long)]
        id: u64,
    },
}

fn main() {
    let args = Cli::parse();

    let conn = match Connection::open("data/prm.db") {
        Ok(conn) => conn,
        Err(_) => {
            eprintln!("Error opening database");
            exit(exitcode::UNAVAILABLE);
        }
    };

    match args.command {
        Commands::Init {} => {
            match prm::db::db_helpers::init_db(&conn) {
                Ok(_) => println!("Database initialised"),
                Err(_) => {
                    eprintln!("Error initalising database");
                    exit(exitcode::UNAVAILABLE);
                }
            };
        }
        Commands::Add(add) => match add.entity {
            AddEntity::Person {
                name,
                birthday,
                contact_info,
            } => {
                if let Err(e) = cli::add::person(&conn, name, birthday, contact_info) {
                    eprintln!("{}", e);
                    exit(exitcode::DATAERR);
                };
            }
            AddEntity::Activity {
                name,
                activity_type,
                date,
                content,
                people,
            } => {
                if let Err(e) =
                    cli::add::activity(&conn, name, activity_type, date, content, people)
                {
                    eprintln!("{}", e);
                    exit(exitcode::DATAERR);
                };
            }
            AddEntity::Reminder {
                name,
                date,
                recurring,
                description,
                people,
            } => {
                if let Err(e) =
                    cli::add::reminder(&conn, name, date, recurring, description, people)
                {
                    eprintln!("{}", e);
                    exit(exitcode::DATAERR);
                };
            }
            AddEntity::Notes { content, people } => {
                match cli::add::note(&conn, content, people) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error adding note");
                        exit(exitcode::DATAERR);
                    }
                };
            }
        },
        Commands::Show(show) => match show.entity {
            ShowEntity::Person { name, birthday } => {
                if [name.clone(), birthday.clone()].iter().all(Option::is_none) {
                    eprintln!("No name or birthday provided");
                    exit(exitcode::DATAERR);
                }
                let person = match Person::get_by_name(&conn, name, birthday) {
                    Ok(person) => match person {
                        Some(person) => person,
                        None => {
                            eprintln!("Person not found");
                            exit(exitcode::DATAERR);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error while fecthing person: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                println!("got person: {}", person);
            }
            ShowEntity::Activity { name, person } => {
                // TODO likely useful to return a vector of activities
                if [name.clone(), person.clone()].iter().all(Option::is_none) {
                    eprintln!("No name or person provided");
                    exit(exitcode::DATAERR);
                }
                let activity = match Activity::get(&conn, name, person) {
                    Ok(activity) => activity,
                    Err(_) => {
                        eprintln!("Activity not found");
                        exit(exitcode::SOFTWARE);
                    }
                };
                println!("got activity: {:#?}", activity);
            }
            ShowEntity::Reminder { name } => {
                let reminder = match Reminder::get_by_name(&conn, &name) {
                    Ok(reminder) => match reminder {
                        Some(reminder) => reminder,
                        None => {
                            eprintln!("Reminder not found");
                            exit(exitcode::DATAERR);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error fetching reminder: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                println!("got reminder: {:#?}", reminder);
            }
            ShowEntity::Notes { person } => {
                let note = Note::get_by_person(&conn, person);
                println!("got note: {:#?}", note);
            }
        },
        Commands::Edit(edit) => match edit.entity {
            EditEntity::Person {
                id,
                name,
                birthday,
                contact_info,
            } => {
                match cli::edit::person(&conn, id, name, birthday, contact_info) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing person");
                        exit(exitcode::DATAERR);
                    }
                };
            }
            EditEntity::Activity {
                id,
                name,
                activity_type,
                date,
                content,
            } => {
                match cli::edit::activity(&conn, id, name, activity_type, date, content) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing activity");
                        exit(exitcode::DATAERR);
                    }
                };
            }
            EditEntity::Reminder {
                id,
                name,
                date,
                description,
                recurring,
            } => {
                match cli::edit::reminder(&conn, id, name, date, description, recurring) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing reminder");
                        exit(exitcode::DATAERR);
                    }
                };
            }
            EditEntity::Note { id, date, content } => {
                match cli::edit::note(&conn, id, date, content) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing note");
                        exit(exitcode::DATAERR);
                    }
                };
            }
        },
        Commands::Remove(remove) => match remove.entity {
            RemoveEntity::Person { name } => {
                let person = match Person::get_by_name(&conn, Some(name), None) {
                    Ok(person) => match person {
                        Some(person) => person,
                        None => {
                            eprintln!("Person not found");
                            exit(exitcode::DATAERR);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error while fecthing person: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                match person.remove(&conn) {
                    Ok(_) => println!("{} removed successfully", person),
                    Err(_) => {
                        eprintln!("Error while removing {}", person);
                        exit(exitcode::DATAERR);
                    }
                };
                println!("removed: {}", person);
            }
            RemoveEntity::Activity { name } => {
                // TODO add filter by person
                let activities = match Activity::get(&conn, Some(name), None) {
                    Ok(activity) => activity,
                    Err(e) => {
                        eprintln!("Error while fetching activity: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                // TODO add a way to select between multiple activities to be removed
                if activities.len() > 1 {
                    eprintln!("Found multiple activities");
                    exit(exitcode::DATAERR);
                }
                match activities[0].remove(&conn) {
                    Ok(_) => println!("{:#?} removed successfully", activities),
                    Err(_) => {
                        eprintln!("Error while removing {:#?}", activities);
                        exit(exitcode::DATAERR);
                    }
                };
                println!("removed: {:#?}", activities);
            }
            RemoveEntity::Reminder { name } => {
                let reminder = match Reminder::get_by_name(&conn, &name) {
                    Ok(reminder) => match reminder {
                        Some(reminder) => reminder,
                        None => {
                            eprintln!("Reminder not found");
                            exit(exitcode::DATAERR);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error fetching reminder: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                match reminder.remove(&conn) {
                    Ok(_) => println!("{:#?} removed successfully", reminder),
                    Err(_) => {
                        eprintln!("Error while removing {:#?}", reminder);
                        exit(exitcode::DATAERR);
                    }
                };
                println!("removed: {:#?}", reminder);
            }
            RemoveEntity::Note { id } => {
                let note = Note::get_by_id(&conn, id);
                match note {
                    Ok(note) => match note {
                        Some(note) => {
                            if let Entities::Note(note) = note {
                                match note.remove(&conn) {
                                    Ok(_) => println!("{:#?} removed successfully", note),
                                    Err(_) => {
                                        eprintln!("Error while removing {:#?}", note);
                                        exit(exitcode::DATAERR);
                                    }
                                };
                                println!("removed: {:#?}", note);
                            }
                        }
                        None => {
                            println!("Could not find note with id: {}", id);
                            return;
                        }
                    },
                    Err(e) => {
                        eprintln!("Error while fetching note: {:#?}", e);
                        return;
                    }
                };
            }
        },
        Commands::List(list) => match list.entity {
            ListEntity::People {} => {
                let people = Person::get_all(&conn);
                if let Ok(person) = people {
                    println!("{:#?}", person);
                }
            }
            ListEntity::Activities {} => {
                let activities = Activity::get_all(&conn);
                println!("listing activities: {:#?}", activities);
            }
            ListEntity::Reminders { include_past } => {
                let reminders = Reminder::get_all(&conn, include_past);
                if let Ok(reminder) = reminders {
                    println!("{:#?}", reminder);
                }
            }
            ListEntity::Notes {} => {
                let notes = Note::get_all(&conn);
                println!("listing notes: {:#?}", notes);
            }
            ListEntity::Events { days } => {
                let mut events = match Event::get_all(&conn, days) {
                    Ok(events) => events,
                    Err(e) => {
                        eprintln!("Error while fetching events: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };

                // Sort events by date (month and day)
                events.sort_by(|a, b| {
                    let a_md = a.date.format("%m-%d").to_string();
                    let b_md = b.date.format("%m-%d").to_string();
                    a_md.cmp(&b_md)
                });
                for event in events {
                    println!("{}", event);
                }
            }
        },
        Commands::Ics(ics) => {
            let events = match Event::get_all(&conn, 0) {
                Ok(events) => events,
                Err(e) => {
                    eprintln!("Error while fetching events: {:#?}", e);
                    exit(exitcode::DATAERR);
                }
            };
            let mut calendar = ICalendar::new("2.0", "ics-rs");

            for event in events {
                let uuid = Uuid::new_v4();
                let dtstamp = chrono::Local::now().format("%Y%m%dT%H%M%SZ").to_string();
                match event.details {
                    EventType::Person(person) => {
                        if !ics.birthdays && !ics.all {
                            continue;
                        }
                        let mut ics_event = IcsEvent::new(uuid.to_string(), dtstamp);
                        let dtstart = format!("{}", event.date.format("%Y%m%d").to_string());
                        ics_event.push(Summary::new(format!("{}'s birthday", person.name)));
                        ics_event.push(Comment::new(escape_text(format!(
                            "Contact info: {:#?}",
                            person.contact_info
                        ))));
                        ics_event.push(DtStart::new(dtstart));
                        ics_event.push(RRule::new("FREQ=YEARLY"));
                        calendar.add_event(ics_event);
                    }
                    EventType::Reminder(reminder) => {
                        // TODO macos reminders.app does not work well with caldav
                        if !ics.reminders && !ics.all {
                            continue;
                        }
                        let mut todo = ToDo::new(uuid.to_string(), dtstamp);
                        let dtdue = format!("{}T090000", event.date.format("%Y%m%d").to_string());
                        todo.push(Summary::new(reminder.name));
                        todo.push(Comment::new(
                            reminder.description.unwrap_or(String::from("[Empty]")),
                        ));
                        todo.push(Status::needs_action());
                        todo.push(Due::new(dtdue));
                        calendar.add_todo(todo);
                    }
                }
            }
            match calendar.save_file("data/calendar.ics") {
                Ok(_) => println!("Saved to data/calendar.ics"),
                Err(e) => {
                    eprintln!("Error while saving to data/calendar.ics: {:#?}", e);
                    exit(exitcode::SOFTWARE);
                }
            };
        }
    }
}
