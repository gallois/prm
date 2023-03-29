use chrono::prelude::*;
use clap::builder::ArgAction;
use clap::{Args, Parser, Subcommand};
use prm::db::db_interface::DbOperations;
use prm::{
    Activity, ActivityType, ContactInfo, ContactInfoType, Note, Person, RecurringType, Reminder,
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
    #[command(arg_required_else_help = true)]
    Edit {
        entity: String,
    },
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
        name: String,
        #[arg(short, long)]
        birthday: Option<String>,
        #[arg(short, long)]
        contact_info: Option<String>,
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
enum ListEntity {
    // TODO add some filtering
    Person,
    Activity,
    Reminders {
        #[arg(short, long, action = ArgAction::SetTrue)]
        include_past: bool,
    },
    Notes,
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
    // Notes {
    //     #[arg(short, long)]
    //     person: String,
    // },
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
                    let mut birthday_obj: Option<NaiveDate> = None;
                    match birthday {
                        Some(birthday_str) => {
                            // TODO proper error handling and messaging
                            match prm::helpers::parse_from_str_ymd(&birthday_str) {
                                Ok(date) => birthday_obj = Some(date),
                                Err(_) => match prm::helpers::parse_from_str_md(&birthday_str) {
                                    Ok(date) => birthday_obj = Some(date),
                                    Err(error) => panic!("Error parsing birthday: {}", error),
                                },
                            }
                        }
                        None => (),
                    }

                    let contact_info_split: Vec<String>;
                    let mut contact_info_type: Option<ContactInfoType> = None;

                    // TODO allow for multiple contact info on creation
                    match contact_info {
                        Some(contact_info_str) => {
                            contact_info_split =
                                contact_info_str.split(":").map(|x| x.to_string()).collect()
                        }
                        None => contact_info_split = vec![],
                    }

                    if contact_info_split.len() > 0 {
                        match contact_info_split[0].as_str() {
                            "phone" => {
                                contact_info_type =
                                    Some(ContactInfoType::Phone(contact_info_split[1].clone()))
                            }
                            "whatsapp" => {
                                contact_info_type =
                                    Some(ContactInfoType::WhatsApp(contact_info_split[1].clone()))
                            }
                            "email" => {
                                contact_info_type =
                                    Some(ContactInfoType::Email(contact_info_split[1].clone()))
                            }
                            // TODO proper error handling and messaging
                            _ => panic!("Unknown contact info type"),
                        }
                    }

                    let contact_info: Vec<ContactInfo> = Vec::new();

                    let person = Person::new(0, name, birthday_obj, contact_info);
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
                        _ => panic!("Unknown activity type"),
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
                let person = prm::Person::get_by_name(&conn, &name).unwrap();
                println!("got person: {:#?}", person);
            }
            ShowEntity::Activity { name } => {
                // TODO likely useful to return a vector of activities
                let activity = prm::Activity::get_by_name(&conn, &name).unwrap();
                println!("got activity: {:#?}", activity);
            }
            ShowEntity::Reminder { name } => {
                let reminder = prm::Reminder::get_by_name(&conn, &name).unwrap();
                println!("got reminder: {:#?}", reminder);
            }
            ShowEntity::Notes { person } => {
                let note = prm::Note::get_by_person(&conn, person);
                println!("got note: {:#?}", note);
            }
        },
        Commands::Edit { entity } => {
            println!("Editing {}", entity);
        }
        Commands::Remove(remove) => match remove.entity {
            RemoveEntity::Person { name } => {
                let person = prm::Person::get_by_name(&conn, &name).unwrap();
                match person.remove(&conn) {
                    Ok(_) => println!("{:#?} removed successfully", person),
                    Err(_) => panic!("Error while removing {:#?}", person),
                };
                println!("removed: {:#?}", person);
            }
            RemoveEntity::Activity { name } => {
                let activity = prm::Activity::get_by_name(&conn, &name).unwrap();
                match activity.remove(&conn) {
                    Ok(_) => println!("{:#?} added successfully", activity),
                    Err(_) => panic!("Error while adding {:#?}", activity),
                };
                println!("removed: {:#?}", activity);
            }
            RemoveEntity::Reminder { name } => {
                let reminder = prm::Reminder::get_by_name(&conn, &name).unwrap();
                match reminder.remove(&conn) {
                    Ok(_) => println!("{:#?} added successfully", reminder),
                    Err(_) => panic!("Error while adding {:#?}", reminder),
                };
                println!("removed: {:#?}", reminder);
            }
        },
        Commands::List(list) => match list.entity {
            ListEntity::Person {} => {
                let people = Person::get_all(&conn);
                println!("listing people: {:#?}", people);
            }
            ListEntity::Activity {} => {
                let activities = Activity::get_all(&conn);
                println!("listing activities: {:#?}", activities);
            }
            ListEntity::Reminders { include_past } => {
                let reminders = Reminder::get_all(&conn, include_past);
                println!("listing reminders: {:#?}", reminders);
            }
            ListEntity::Notes {} => {
                let notes = Note::get_all(&conn);
                println!("listing notes: {:#?}", notes);
            }
        },
    }
}
