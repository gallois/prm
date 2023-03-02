use chrono::prelude::*;
use clap::{Parser, Subcommand};

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
        #[arg(required = false)]
        name: String,
        #[arg(required = false)]
        birthday: String,
        #[arg(required = false)]
        contact_info: String,
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
                let birthday = NaiveDate::parse_from_str(&birthday, "%Y-%m-%d").unwrap();
                let contact_info_split: Vec<&str> = contact_info.split(":").collect();
                let contact_info_type;

                match contact_info_split[0] {
                    "phone" => {
                        contact_info_type =
                            prm::ContactInfoType::Phone(String::from(contact_info_split[1]))
                    }
                    "whatsapp" => {
                        contact_info_type =
                            prm::ContactInfoType::Whatsapp(String::from(contact_info_split[1]))
                    }
                    "email" => {
                        contact_info_type =
                            prm::ContactInfoType::Email(String::from(contact_info_split[1]))
                    }
                    _ => panic!("Unknown contact info type"),
                }

                let contact_info = prm::ContactInfo {
                    contact_info_type: contact_info_type,
                };

                let person = prm::Person::new(name, birthday, vec![contact_info]);
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
