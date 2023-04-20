use chrono::prelude::*;
use clap::builder::ArgAction;
use clap::{Args, Parser, Subcommand};
use edit;
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
        Commands::Add(add) => {
            match add.entity {
                AddEntity::Person {
                    name,
                    birthday,
                    contact_info,
                } => {
                    let mut name_str: String = String::new();
                    let mut birthday_str: Option<String> = None;
                    let mut contact_info_vec: Vec<String> = vec![];
                    let mut editor = false;
                    if name == None {
                        editor = true;
                        let edited = edit::edit(prm::PERSON_TEMPLATE).unwrap();
                        let (n, b, c) = match Person::parse_from_editor(edited.as_str()) {
                            Ok((person, birthday, contact_info)) => {
                                (person, birthday, contact_info)
                            }
                            Err(_) => panic!("Error parsing person"),
                        };
                        name_str = n;
                        birthday_str = b;
                        contact_info_vec = c;
                    }

                    if !editor {
                        name_str = name.unwrap();
                    }
                    let mut birthday_obj: Option<NaiveDate> = None;
                    if !editor {
                        if let Some(bday) = birthday {
                            birthday_str = Some(bday);
                        }
                    }

                    if let Some(birthday_str) = birthday_str {
                        match prm::helpers::parse_from_str_ymd(&birthday_str) {
                            // TODO proper error handling and messaging
                            Ok(date) => birthday_obj = Some(date),
                            Err(_) => match prm::helpers::parse_from_str_md(&birthday_str) {
                                Ok(date) => birthday_obj = Some(date),
                                Err(error) => panic!("Error parsing birthday: {}", error),
                            },
                        }
                    }

                    let mut contact_info_splits: Vec<Vec<String>> = vec![];
                    let mut contact_info_types: Vec<ContactInfoType> = vec![];

                    match contact_info {
                        Some(mut contact_info_vec) => {
                            if !editor {
                                ContactInfo::populate_splits(
                                    &mut contact_info_splits,
                                    &mut contact_info_vec,
                                );
                            }
                        }
                        None => {
                            if editor {
                                ContactInfo::populate_splits(
                                    &mut contact_info_splits,
                                    &mut contact_info_vec,
                                );
                            }
                        }
                    }

                    if contact_info_splits.len() > 0 {
                        contact_info_splits
                            .into_iter()
                            .for_each(|contact_info_split| {
                                match contact_info_split[0].as_str() {
                                    "phone" => contact_info_types.push(ContactInfoType::Phone(
                                        contact_info_split[1].clone(),
                                    )),
                                    "whatsapp" => contact_info_types.push(
                                        ContactInfoType::WhatsApp(contact_info_split[1].clone()),
                                    ),
                                    "email" => contact_info_types.push(ContactInfoType::Email(
                                        contact_info_split[1].clone(),
                                    )),
                                    // TODO proper error handling and messaging
                                    _ => panic!("Unknown contact info type"),
                                }
                            });
                    }

                    let mut contact_info: Vec<ContactInfo> = Vec::new();
                    if contact_info_types.len() > 0 {
                        contact_info_types
                            .into_iter()
                            .for_each(|contact_info_type| {
                                contact_info.push(ContactInfo::new(0, 0, contact_info_type));
                            });
                    }

                    assert_eq!(name_str.is_empty(), false, "Name cannot be empty");
                    let person = Person::new(0, name_str, birthday_obj, contact_info);
                    match person.add(&conn) {
                        Ok(_) => println!("{:#?} added successfully", person),
                        Err(_) => panic!("Error while adding {:#?}", person),
                    };
                }
                AddEntity::Activity {
                    name,
                    activity_type,
                    date,
                    content,
                    people,
                } => {
                    let activity_type = match activity_type.as_str() {
                        "phone" => ActivityType::Phone,
                        "in_person" => ActivityType::InPerson,
                        "online" => ActivityType::Online,
                        // TODO proper error handling and messaging
                        _ => panic!("Unknown reminder type"),
                    };

                    let date_obj = match prm::helpers::parse_from_str_ymd(date.as_str()) {
                        Ok(date) => date,
                        Err(error) => panic!("Error parsing date: {}", error),
                    };

                    let people = Person::get_by_names(&conn, people);

                    let activity = Activity::new(0, name, activity_type, date_obj, content, people);
                    match activity.add(&conn) {
                        Ok(_) => println!("{:#?} added successfully", activity),
                        Err(_) => panic!("Error while adding {:#?}", activity),
                    };
                }
                AddEntity::Reminder {
                    name,
                    date,
                    recurring,
                    description,
                    people,
                } => {
                    let recurring_type = match recurring {
                        Some(recurring_type_str) => match recurring_type_str.as_str() {
                            "daily" => Some(RecurringType::Daily),
                            "weekly" => Some(RecurringType::Weekly),
                            "fortnightly" => Some(RecurringType::Fortnightly),
                            "monthly" => Some(RecurringType::Monthly),
                            "quarterly" => Some(RecurringType::Quarterly),
                            "biannual" => Some(RecurringType::Biannual),
                            "yearly" => Some(RecurringType::Yearly),
                            _ => panic!("Unknown recurring pattern"),
                        },
                        None => None,
                    };

                    let date_obj = match prm::helpers::parse_from_str_ymd(date.as_str()) {
                        Ok(date) => date,
                        Err(error) => panic!("Error parsing date: {}", error),
                    };

                    let people = Person::get_by_names(&conn, people);

                    let reminder =
                        Reminder::new(0, name, date_obj, description, recurring_type, people);
                    println!("Reminder: {:#?}", reminder);
                    match reminder.add(&conn) {
                        Ok(_) => println!("{:#?} added successfully", reminder),
                        Err(_) => panic!("Error while adding {:#?}", reminder),
                    };
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
            }
        }
        Commands::Show(show) => match show.entity {
            ShowEntity::Person { name } => {
                let person = Person::get_by_name(&conn, &name).unwrap();
                println!("got person: {:#?}", person);
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
                                    format!("Failed to update person: {:#?}", person).as_str(),
                                );
                                println!("Updated person: {:#?}", person);
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
                    Ok(_) => println!("{:#?} removed successfully", person),
                    Err(_) => panic!("Error while removing {:#?}", person),
                };
                println!("removed: {:#?}", person);
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

                // Sort events by date (month and day) using the `sort_by` function.
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
