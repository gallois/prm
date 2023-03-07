use chrono::prelude::*;
use clap::{Parser, Subcommand};
use prm::{ContactInfo, ContactInfoType};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Add {
        entity: String,
        #[arg(required = true)]
        name: String,
        #[arg(short, long, required = false)]
        birthday: Option<String>,
        #[arg(short, long, required = false)]
        contact_info: Option<String>,
    },
    #[command(arg_required_else_help = true)]
    Show { entity: String },
    #[command(arg_required_else_help = true)]
    Edit { entity: String },
    #[command(arg_required_else_help = true)]
    Remove { entity: String },
    #[command(arg_required_else_help = true)]
    List { entity: String },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Add {
            entity,
            name,
            birthday,
            contact_info,
        } => match entity.as_str() {
            "person" => {
                let mut birthday_obj: Option<NaiveDate> = None;
                match birthday {
                    Some(birthday_str) => {
                        let birthday_result = NaiveDate::parse_from_str(&birthday_str, "%Y-%m-%d");
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
                    Some(contact_info_type) => contact_info.push(ContactInfo { contact_info_type }),
                    None => (),
                }

                let person = prm::Person::new(name, birthday_obj, contact_info);
                println!("Person: {:#?}", person);
            }
            other => {
                println!("Entity unknown: {other}");
            }
        },
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
