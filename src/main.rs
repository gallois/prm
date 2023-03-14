use chrono::prelude::*;
use clap::{Args, Parser, Subcommand};
use prm::{
    Activity, ActivityType, ContactInfo, ContactInfoType, DbOperations, Notes, Person,
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
    #[command(arg_required_else_help = true)]
    Show {
        entity: String,
    },
    #[command(arg_required_else_help = true)]
    Edit {
        entity: String,
    },
    #[command(arg_required_else_help = true)]
    Remove {
        entity: String,
    },
    #[command(arg_required_else_help = true)]
    List {
        entity: String,
    },
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
struct AddArgs {
    #[command(subcommand)]
    entity: Entity,
}

#[derive(Subcommand)]
enum Entity {
    Person {
        name: String,
        #[arg(short, long, required = false)]
        birthday: Option<String>,
        #[arg(short, long, required = false)]
        contact_info: Option<String>,
    },
    Activity {
        name: String,
        #[arg(short, long, required = true)]
        activity_type: String,
        #[arg(short, long, required = true)]
        date: String,
        #[arg(short, long, required = true)]
        content: String,
    },
    Reminder {
        name: String,
        #[arg(short, long, required = true)]
        date: String,
        #[arg(short, long, required = false)]
        recurring: Option<String>,
        #[arg(long, required = false)]
        description: Option<String>,
        // TODO add person
    },
    Notes {
        content: String,
    },
}

fn main() {
    let args = Cli::parse();

    let conn = Connection::open("data/prm.db").unwrap();

    match args.command {
        Commands::Init {} => {
            match prm::init_db(&conn) {
                Ok(_) => println!("Database initialised"),
                Err(_) => panic!("Error initialising database"),
            };
        }
        Commands::Add(add) => {
            match add.entity {
                Entity::Person {
                    name,
                    birthday,
                    contact_info,
                } => {
                    let mut birthday_obj: Option<NaiveDate> = None;
                    match birthday {
                        Some(birthday_str) => {
                            // TODO extract common logic, e.g. parsing dates
                            let birthday_result =
                                NaiveDate::parse_from_str(&birthday_str, "%Y-%m-%d");
                            // TODO allow for entering date without year
                            // TODO proper error handling and messaging
                            match birthday_result {
                                Ok(date) => birthday_obj = Some(date),
                                Err(error) => panic!("Error parsing birthday: {}", error),
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

                    let mut contact_info: Vec<ContactInfo> = Vec::new();
                    match contact_info_type {
                        Some(contact_info_type) => {
                            contact_info.push(ContactInfo { contact_info_type })
                        }
                        None => (),
                    }

                    let person = Person::new(name, birthday_obj, contact_info);
                    println!("[DEBUG] Person: {:#?}", person);
                    match person.add(&conn) {
                        Ok(_) => println!("{:#?} added successfully", person),
                        Err(_) => panic!("Error while adding {:#?}", person),
                    };
                }
                // TODO will require linking to a person
                Entity::Activity {
                    name,
                    activity_type,
                    date,
                    content,
                } => {
                    let activity_type = match activity_type.as_str() {
                        "phone" => ActivityType::Phone,
                        "in_person" => ActivityType::InPerson,
                        "online" => ActivityType::Online,
                        // TODO proper error handling and messaging
                        _ => panic!("Unknown activity type"),
                    };

                    let date_obj = match NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d") {
                        Ok(date) => date,
                        Err(error) => panic!("Error parsing date: {}", error),
                    };

                    let activity = Activity::new(name, activity_type, date_obj, content, vec![]);
                    println!("Activity: {:#?}", activity);
                    match activity.add(&conn) {
                        Ok(_) => println!("{:#?} added successfully", activity),
                        Err(_) => panic!("Error while adding {:#?}", activity),
                    };
                }
                // TODO link to people
                Entity::Reminder {
                    name,
                    date,
                    recurring,
                    description,
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

                    let date_obj = match NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d") {
                        Ok(date) => date,
                        Err(error) => panic!("Error parsing date: {}", error),
                    };

                    let reminder =
                        Reminder::new(name, date_obj, description, recurring_type, vec![]);
                    println!("Reminder: {:#?}", reminder);
                    match reminder.add(&conn) {
                        Ok(_) => println!("{:#?} added successfully", reminder),
                        Err(_) => panic!("Error while adding {:#?}", reminder),
                    };
                }
                Entity::Notes { content } => {
                    let date = Utc::now().date_naive();

                    let note = Notes::new(date, content, vec![]);
                    println!("Note: {:#?}", note);
                    match note.add(&conn) {
                        Ok(_) => println!("{:#?} added successfully", note),
                        Err(_) => panic!("Error while adding {:#?}", note),
                    };
                }
            }
        }
        Commands::Show { entity } => {
            println!("Showing {}", entity);
        }
        Commands::Edit { entity } => {
            println!("Editing {}", entity);
        }
        Commands::Remove { entity } => {
            println!("Removing {}", entity);
        }
        Commands::List { entity } => {
            println!("Listing {}", entity);
        }
    }
}
