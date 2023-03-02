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
    Add { entity: String },
    #[command(arg_required_else_help = true)]
    Show { entity: String },
    #[command(arg_required_else_help = true)]
    Edit { entity: String },
    #[command(arg_required_else_help = true)]
    Remove { entity: String },
    #[command(arg_required_else_help = true)]
    List { entity: String },
}

struct Person {
    name: String,
    birthday: DateTime<Utc>,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
}

struct Activity {
    activity_type: ActivityType,
    name: String,
    date: DateTime<Utc>,
    content: String,
}

enum ActivityType {
    Phone,
    InPerson,
    Online,
}

struct Reminder {
    name: String,
    date: DateTime<Utc>,
    recurring: Option<RecurringType>,
    people: Vec<Person>,
}

enum RecurringType {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Biannual,
    Yearly,
}

struct ContactInfo {
    contact_info_type: ContactInfoType,
}

enum ContactInfoType {
    Phone(String),
    Whatsapp(String),
    Email(String),
}

enum Entity {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Add { entity } => {
            println!("Adding {}", entity);
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
