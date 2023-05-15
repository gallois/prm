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
        name: String,
        // TODO Filter by birthday etc.
    },
    Activity {
        #[arg(short, long)]
        name: String,
        // TODO Filter by people etc.
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

    let conn = Connection::open("data/prm.db").unwrap();
    // let conn = prm::db::conn_init(prm::db::DbConnectionType::Real);

    match args.command {
        Commands::Init {} => {
            match prm::db::db_helpers::init_db(&conn) {
                Ok(_) => println!("Database initialised"),
                Err(_) => panic!("Error initialising database"),
            };
        }
        Commands::Add(add) => match add.entity {
            AddEntity::Person {
                name,
                birthday,
                contact_info,
            } => {
                cli::add::person(&conn, name, birthday, contact_info);
            }
            AddEntity::Activity {
                name,
                activity_type,
                date,
                content,
                people,
            } => {
                cli::add::activity(&conn, name, activity_type, date, content, people);
            }
            AddEntity::Reminder {
                name,
                date,
                recurring,
                description,
                people,
            } => {
                cli::add::reminder(&conn, name, date, recurring, description, people);
            }
            AddEntity::Notes { content, people } => {
                cli::add::note(&conn, content, people);
            }
        },
        Commands::Show(show) => match show.entity {
            ShowEntity::Person { name } => {
                let person = Person::get_by_name(&conn, &name).unwrap();
                println!("got person: {}", person);
            }
            ShowEntity::Activity { name } => {
                // TODO likely useful to return a vector of activities
                let reminder = Activity::get_by_name_new(&conn, &name).unwrap();
                println!("got reminder: {:#?}", reminder);
            }
            ShowEntity::Reminder { name } => {
                let reminder = Reminder::get_by_name(&conn, &name).unwrap();
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
                cli::edit::person(&conn, id, name, birthday, contact_info);
            }
            EditEntity::Activity {
                id,
                name,
                activity_type,
                date,
                content,
            } => {
                cli::edit::activity(&conn, id, name, activity_type, date, content);
            }
            EditEntity::Reminder {
                id,
                name,
                date,
                description,
                recurring,
            } => {
                cli::edit::reminder(&conn, id, name, date, description, recurring);
            }
            EditEntity::Note { id, date, content } => {
                cli::edit::note(&conn, id, date, content);
            }
        },
        Commands::Remove(remove) => match remove.entity {
            RemoveEntity::Person { name } => {
                let person = Person::get_by_name(&conn, &name).unwrap();
                match person.remove(&conn) {
                    Ok(_) => println!("{} removed successfully", person),
                    Err(_) => panic!("Error while removing {}", person),
                };
                println!("removed: {}", person);
            }
            RemoveEntity::Activity { name } => {
                let reminder = Activity::get_by_name(&conn, &name).unwrap();
                match reminder.remove(&conn) {
                    Ok(_) => println!("{:#?} removed successfully", reminder),
                    Err(_) => panic!("Error while removing {:#?}", reminder),
                };
                println!("removed: {:#?}", reminder);
            }
            RemoveEntity::Reminder { name } => {
                let reminder = Reminder::get_by_name(&conn, &name).unwrap();
                match reminder.remove(&conn) {
                    Ok(_) => println!("{:#?} removed successfully", reminder),
                    Err(_) => panic!("Error while removing {:#?}", reminder),
                };
                println!("removed: {:#?}", reminder);
            }
            RemoveEntity::Note { id } => {
                let note = Note::get_by_id(&conn, id);
                match note {
                    Some(note) => {
                        if let Entities::Note(note) = note {
                            match note.remove(&conn) {
                                Ok(_) => println!("{:#?} removed successfully", note),
                                Err(_) => panic!("Error while removing {:#?}", note),
                            };
                            println!("removed: {:#?}", note);
                        }
                    }
                    None => {
                        println!("Could not find note with id: {}", id);
                        return;
                    }
                };
            }
        },
        Commands::List(list) => match list.entity {
            ListEntity::People {} => {
                let people = Person::get_all(&conn);
                for person in people {
                    println!("{}", person);
                }
            }
            ListEntity::Activities {} => {
                let activities = Activity::get_all(&conn);
                println!("listing activities: {:#?}", activities);
            }
            ListEntity::Reminders { include_past } => {
                let reminders = Reminder::get_all(&conn, include_past);
                for reminder in reminders {
                    println!("{}", reminder);
                }
            }
            ListEntity::Notes {} => {
                let notes = Note::get_all(&conn);
                println!("listing notes: {:#?}", notes);
            }
            ListEntity::Events { days } => {
                let mut events = Event::get_all(&conn, days);

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
            let events = Event::get_all(&conn, 0);
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
            calendar.save_file("data/calendar.ics").unwrap();
        }
    }
}
