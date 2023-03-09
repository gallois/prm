use chrono::prelude::*;
use clap::{Args, Parser, Subcommand};
use prm::{ContactInfo, ContactInfoType};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
}

fn main() {
    let args = Cli::parse();

    match args.command {
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
                                    Some(prm::ContactInfoType::Phone(contact_info_split[1].clone()))
                            }
                            "whatsapp" => {
                                contact_info_type = Some(prm::ContactInfoType::Whatsapp(
                                    contact_info_split[1].clone(),
                                ))
                            }
                            "email" => {
                                contact_info_type =
                                    Some(prm::ContactInfoType::Email(contact_info_split[1].clone()))
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

                    let person = prm::Person::new(name, birthday_obj, contact_info);
                    println!("Person: {:#?}", person);
                }
                // TODO will require linking to a person
                Entity::Activity {
                    name,
                    activity_type,
                    date,
                    content,
                } => {
                    let activity_type = match activity_type.as_str() {
                        "phone" => prm::ActivityType::Phone,
                        "in_person" => prm::ActivityType::InPerson,
                        "online" => prm::ActivityType::Online,
                        // TODO proper error handling and messaging
                        _ => panic!("Unknown activity type"),
                    };

                    let date_obj = match NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d") {
                        Ok(date) => date,
                        Err(error) => panic!("Error parsing date: {}", error),
                    };

                    let activity = prm::Activity::new(name, activity_type, date_obj, content);
                    println!("Activity: {:#?}", activity);
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
                            "daily" => Some(prm::RecurringType::Daily),
                            "weekly" => Some(prm::RecurringType::Weekly),
                            "fortnightly" => Some(prm::RecurringType::Fortnightly),
                            "monthly" => Some(prm::RecurringType::Monthly),
                            "quarterly" => Some(prm::RecurringType::Quarterly),
                            "biannual" => Some(prm::RecurringType::Biannual),
                            "yearly" => Some(prm::RecurringType::Yearly),
                            _ => panic!("unknown recurring pattern"),
                        },
                        None => None,
                    };

                    let date_obj = match NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d") {
                        Ok(date) => date,
                        Err(error) => panic!("Error parsing date: {}", error),
                    };

                    let reminder =
                        prm::Reminder::new(name, date_obj, recurring_type, vec![], description);
                    println!("Reminder: {:#?}", reminder);
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
