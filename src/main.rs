use chrono::prelude::*;
use clap::builder::ArgAction;
use clap::{Args, Parser, Subcommand};
use prm::db::db_interface::DbOperations;
use prm::{
    Activity, ActivityType, ContactInfo, ContactInfoType, Entities, Event, Note, Person,
    RecurringType, Reminder,
};
use rusqlite::Connection;

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
        name: String,
        #[arg(short, long)]
        activity_type: String,
        #[arg(short, long)]
        date: String,
        #[arg(short, long)]
        content: String,
        #[arg(short, long)]
        people: Vec<String>,
    },
    Reminder {
        name: String,
        #[arg(short, long)]
        date: String,
        #[arg(short, long)]
        recurring: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(short, long)]
        people: Vec<String>,
    },
    Notes {
        content: String,
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
                prm::cli::add::person(&conn, name, birthday, contact_info);
            }
            AddEntity::Activity {
                name,
                activity_type,
                date,
                content,
                people,
            } => {
                prm::cli::add::activity(&conn, name, activity_type, date, content, people);
            }
            AddEntity::Reminder {
                name,
                date,
                recurring,
                description,
                people,
            } => {
                prm::cli::add::reminder(&conn, name, date, recurring, description, people);
            }
            AddEntity::Notes { content, people } => {
                let date = Utc::now().date_naive();

                let people = Person::get_by_names(&conn, people);

                let note = Note::new(0, date, content, people);
                println!("Note: {:#?}", note);
                match note.add(&conn) {
                    Ok(_) => println!("{:#?} added successfully", note),
                    Err(_) => panic!("Error while adding {:#?}", note),
                };
            }
        },
        Commands::Show(show) => match show.entity {
            ShowEntity::Person { name } => {
                let person = Person::get_by_name(&conn, &name).unwrap();
                println!("got person: {}", person);
            }
            ShowEntity::Activity { name } => {
                // TODO likely useful to return a vector of activities
                let reminder = Activity::get_by_name(&conn, &name).unwrap();
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
        Commands::Edit(edit) => {
            match edit.entity {
                EditEntity::Person {
                    id,
                    name,
                    birthday,
                    contact_info,
                } => {
                    let person = Person::get_by_id(&conn, id);

                    match person {
                        Some(person) => {
                            if [name.clone(), birthday.clone(), contact_info.clone()]
                                .iter()
                                .all(Option::is_none)
                            {
                                println!("You must set at least one of `name`, `birthday` or `contact_info`");
                                return;
                            }
                            if let Entities::Person(mut person) = person {
                                person.update(name, birthday, contact_info);
                                person.save(&conn).expect(
                                    format!("Failed to update person: {}", person).as_str(),
                                );
                                println!("Updated person: {}", person);
                            }
                        }
                        None => {
                            println!("Could not find person id {}", id);
                            return;
                        }
                    }
                }
                EditEntity::Activity {
                    id,
                    name,
                    activity_type,
                    date,
                    content,
                } => {
                    let reminder = Activity::get_by_id(&conn, id);

                    match reminder {
                        Some(reminder) => {
                            if [
                                name.clone(),
                                activity_type.clone(),
                                date.clone(),
                                content.clone(),
                            ]
                            .iter()
                            .all(Option::is_none)
                            {
                                println!("You must set at least one of `name`, `activity_type`, `date' or `content`");
                                return;
                            }
                            if let Entities::Activity(mut reminder) = reminder {
                                reminder.update(name, activity_type, date, content);
                                reminder.save(&conn).expect(
                                    format!("Failed to update reminder: {:#?}", reminder).as_str(),
                                );
                                println!("Updated reminder: {:#?}", reminder);
                            }
                        }
                        None => {
                            println!("Could not find reminder id {}", id);
                            return;
                        }
                    }
                }
                EditEntity::Reminder {
                    id,
                    name,
                    date,
                    description,
                    recurring,
                } => {
                    let reminder = Reminder::get_by_id(&conn, id);

                    match reminder {
                        Some(reminder) => {
                            if [
                                name.clone(),
                                date.clone(),
                                description.clone(),
                                recurring.clone(),
                            ]
                            .iter()
                            .all(Option::is_none)
                            {
                                println!("You must set at least one of `name`, `date`, `description' or `recurring`");
                                return;
                            }
                            if let Entities::Reminder(mut reminder) = reminder {
                                reminder.update(name, date, description, recurring);
                                reminder.save(&conn).expect(
                                    format!("Failed to update reminder: {:#?}", reminder).as_str(),
                                );
                                println!("Updated reminder: {:#?}", reminder);
                            }
                        }
                        None => {
                            println!("Could not find reminder id {}", id);
                            return;
                        }
                    }
                }
                EditEntity::Note { id, date, content } => {
                    let note = Note::get_by_id(&conn, id);

                    match note {
                        Some(note) => {
                            if [date.clone(), content.clone()].iter().all(Option::is_none) {
                                println!("You must set at least one of `date` or `content`");
                            }
                            if let Entities::Note(mut note) = note {
                                note.update(date, content);
                                note.save(&conn)
                                    .expect(format!("Failed to update note: {:#?}", note).as_str());
                                println!("Updated note: {:#?}", note);
                            }
                        }
                        None => {
                            println!("Could not find note id {}", id);
                            return;
                        }
                    }
                }
            }
        }
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
    }
}
