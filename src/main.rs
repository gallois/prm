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
