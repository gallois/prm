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
use prm::helpers::handle_id_selection;
use rusqlite::Connection;
use std::io;
use std::io::Write;
use uuid::Uuid;

use std::process::exit;

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
        name: Option<String>,
        #[arg(short, long)]
        birthday: Option<String>,
    },
    Activity {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        person: Option<String>,
        #[arg(short, long)]
        content: Option<String>,
    },
    Reminder {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        person: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
    Notes {
        #[arg(short, long)]
        person: Option<String>,
        #[arg(short, long)]
        content: Option<String>,
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
    People {
        #[arg(short, long)]
        name: Option<String>,
    },
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
        #[arg(short, long)]
        person: Option<String>,
        #[arg(short, long)]
        content: Option<String>,
    },
    Reminder {
        #[arg(short, long)]
        name: String,
    },
    Note {
        #[arg(short, long)]
        content: String,
    },
}

fn main() {
    let args = Cli::parse();

    let conn = match Connection::open("data/prm.db") {
        Ok(conn) => conn,
        Err(_) => {
            eprintln!("Error opening database");
            exit(exitcode::UNAVAILABLE);
        }
    };

    match args.command {
        Commands::Init {} => {
            match prm::db::db_helpers::init_db(&conn) {
                Ok(_) => println!("Database initialised"),
                Err(_) => {
                    eprintln!("Error initalising database");
                    exit(exitcode::UNAVAILABLE);
                }
            };
        }
        Commands::Add(add) => match add.entity {
            AddEntity::Person {
                name,
                birthday,
                contact_info,
            } => {
                if let Err(e) = cli::add::person(&conn, name, birthday, contact_info) {
                    eprintln!("{}", e);
                    exit(exitcode::DATAERR);
                };
            }
            AddEntity::Activity {
                name,
                activity_type,
                date,
                content,
                people,
            } => {
                // FIXME people should not be empty
                if let Err(e) =
                    cli::add::activity(&conn, name, activity_type, date, content, people)
                {
                    eprintln!("{}", e);
                    exit(exitcode::DATAERR);
                };
            }
            AddEntity::Reminder {
                name,
                date,
                recurring,
                description,
                people,
            } => {
                if let Err(e) =
                    cli::add::reminder(&conn, name, date, recurring, description, people)
                {
                    eprintln!("{}", e);
                    exit(exitcode::DATAERR);
                };
            }
            AddEntity::Notes { content, people } => {
                match cli::add::note(&conn, content, people) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error adding note");
                        exit(exitcode::DATAERR);
                    }
                };
            }
        },
        Commands::Show(show) => match show.entity {
            ShowEntity::Person { name, birthday } => {
                if [name.clone(), birthday.clone()].iter().all(Option::is_none) {
                    eprintln!("No name or birthday provided");
                    exit(exitcode::DATAERR);
                }
                let people = match Person::get_by_name(&conn, name, birthday) {
                    Ok(people) => people,
                    Err(e) => {
                        eprintln!("Error while fecthing person: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                for person in people {
                    println!("{}", person);
                }
            }
            ShowEntity::Activity {
                name,
                person,
                content,
            } => {
                if [name.clone(), person.clone(), content.clone()]
                    .iter()
                    .all(Option::is_none)
                {
                    eprintln!("No name, person or content provided");
                    exit(exitcode::DATAERR);
                }
                let activities = match Activity::get(&conn, name, person, content) {
                    Ok(activities) => activities,
                    Err(e) => {
                        eprintln!("Error fetching activities: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                if activities.is_empty() {
                    println!("No activities found");
                } else {
                    for activity in activities {
                        println!("{}", activity);
                    }
                }
            }
            ShowEntity::Reminder {
                name,
                person,
                description,
            } => {
                if [name.clone(), person.clone(), description.clone()]
                    .iter()
                    .all(Option::is_none)
                {
                    eprintln!("No name, person or description provided");
                    exit(exitcode::DATAERR);
                }
                let reminders = match Reminder::get(&conn, name, person, description) {
                    Ok(reminder) => reminder,
                    Err(e) => {
                        eprintln!("Error fetching reminders: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                if reminders.is_empty() {
                    println!("No reminders found");
                } else {
                    for reminder in reminders {
                        println!("{}", reminder);
                    }
                }
            }
            ShowEntity::Notes { person, content } => {
                if [person.clone(), content.clone()]
                    .iter()
                    .all(Option::is_none)
                {
                    eprintln!("No person or content provided");
                    exit(exitcode::DATAERR);
                }
                let notes = match Note::get(&conn, person, content) {
                    Ok(note) => note,
                    Err(e) => {
                        eprintln!("Error fetching notes: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                if notes.is_empty() {
                    println!("No notes found");
                } else {
                    for note in notes {
                        println!("{}", note);
                    }
                }
            }
        },
        Commands::Edit(edit) => match edit.entity {
            EditEntity::Person {
                id,
                name,
                birthday,
                contact_info,
            } => {
                match cli::edit::person(&conn, id, name, birthday, contact_info) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing person");
                        exit(exitcode::DATAERR);
                    }
                };
            }
            EditEntity::Activity {
                id,
                name,
                activity_type,
                date,
                content,
            } => {
                match cli::edit::activity(&conn, id, name, activity_type, date, content) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing activity");
                        exit(exitcode::DATAERR);
                    }
                };
            }
            EditEntity::Reminder {
                id,
                name,
                date,
                description,
                recurring,
            } => {
                match cli::edit::reminder(&conn, id, name, date, description, recurring) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing reminder");
                        exit(exitcode::DATAERR);
                    }
                };
            }
            EditEntity::Note { id, date, content } => {
                match cli::edit::note(&conn, id, date, content) {
                    Ok(_) => (),
                    Err(_) => {
                        eprintln!("Error editing note");
                        exit(exitcode::DATAERR);
                    }
                };
            }
        },
        Commands::Remove(remove) => match remove.entity {
            RemoveEntity::Person { name } => {
                let mut people = match Person::get_by_name(&conn, Some(name), None) {
                    Ok(people) => people,
                    Err(e) => {
                        eprintln!("Error while fecthing person: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                if people.len() > 1 {
                    eprintln!("Multiple people found");
                    for person in people.clone() {
                        eprintln!("[{}]\n{}", person.id, person);
                    }
                    print!("Which person do you want to remove? ");
                    io::stdout().flush().unwrap();
                    let mut n = String::new();
                    io::stdin().read_line(&mut n).unwrap();
                    let n = n.trim().parse::<usize>().expect("Invalid input");
                    let mut valid_id = false;
                    for person in people.clone() {
                        if person.id == n as u64 {
                            people = vec![person];
                            valid_id = true;
                        }
                    }
                    if !valid_id {
                        eprintln!("Invalid input");
                        exit(exitcode::DATAERR);
                    }
                }

                let person = &people[0];

                println!("{}", person);
                print!("Do you want to remove this person? [y/n] ");
                io::stdout().flush().unwrap();
                let mut answer = String::new();
                io::stdin().read_line(&mut answer).unwrap();
                if answer.trim() != "y" {
                    println!("Not removing");
                    exit(exitcode::OK);
                }

                match person.remove(&conn) {
                    Ok(_) => println!("{}\nremoved successfully", person),
                    Err(_) => {
                        eprintln!("Error while removing {}", person);
                        exit(exitcode::DATAERR);
                    }
                };
            }
            RemoveEntity::Activity {
                name,
                person,
                content,
            } => {
                let mut activities = match Activity::get(&conn, Some(name), person, content) {
                    Ok(activities) => activities,
                    Err(e) => {
                        eprintln!("Error while fetching activity: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                if activities.len() > 1 {
                    println!("Multiple activities found");
                    for activity in activities.clone() {
                        println!("[{}]\n{}", activity.id, activity);
                    }
                    print!("Which activity do you want to remove? ");
                    io::stdout().flush().unwrap();
                    let mut n = String::new();
                    io::stdin().read_line(&mut n).unwrap();
                    let n = n.trim().parse::<usize>().expect("Invalid input");
                    let mut valid_id = false;
                    for activity in activities.clone() {
                        if activity.id == n as u64 {
                            activities = vec![activity];
                            valid_id = true;
                        }
                    }
                    if !valid_id {
                        eprintln!("Invalid input");
                        exit(exitcode::DATAERR);
                    }
                }

                let activity = &activities[0];

                println!("{}", activity);
                print!("Do you want to remove this activity? [y/n] ");
                io::stdout().flush().unwrap();
                let mut answer = String::new();
                io::stdin().read_line(&mut answer).unwrap();
                if answer.trim() != "y" {
                    println!("Not removing");
                    exit(exitcode::OK);
                }

                match activity.remove(&conn) {
                    Ok(_) => println!("{}\nremoved successfully", activity),
                    Err(_) => {
                        eprintln!("Error while removing {}", activity);
                        exit(exitcode::DATAERR);
                    }
                };
            }
            RemoveEntity::Reminder { name } => {
                let mut reminders = match Reminder::get_by_name(&conn, &name, None) {
                    Ok(reminders) => reminders,
                    Err(e) => {
                        eprintln!("Error fetching reminder: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };
                // TODO fix input handling for selecting IDs for other entities and
                //      abstract the implementation, as it's essentially repeated
                //      between them
                if reminders.len() > 1 {
                    reminders = match handle_id_selection::<Reminder>("reminder", reminders) {
                        Ok(reminders) => reminders,
                        Err(e) => {
                            eprintln!("{}", e.message);
                            exit(exitcode::DATAERR);
                        }
                    };
                }

                let reminders = &reminders[0];

                println!("{}", reminders);
                print!("Do you want to remove this reminder? [y/n] ");
                io::stdout().flush().unwrap();
                let mut answer = String::new();
                io::stdin().read_line(&mut answer).unwrap();
                if answer.trim() != "y" {
                    println!("Not removing");
                    exit(0);
                }

                match reminders.remove(&conn) {
                    Ok(_) => println!("{} removed successfully", reminders),
                    Err(_) => {
                        eprintln!("Error while removing {}", reminders);
                        exit(exitcode::DATAERR);
                    }
                };
            }
            RemoveEntity::Note { content } => {
                let mut notes = match Note::get_by_content(&conn, content) {
                    Ok(notes) => notes,
                    Err(e) => {
                        eprintln!("Error while fetching notes: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };

                if notes.len() > 1 {
                    println!("Multiple notes found");
                    for note in notes.clone() {
                        println!("[{}]\n{}", note.id, note);
                    }
                    print!("Which note do you want to remove? ");
                    io::stdout().flush().unwrap();
                    let mut n = String::new();
                    io::stdin().read_line(&mut n).unwrap();
                    let n = n.trim().parse::<usize>().expect("Invalid input");
                    let mut valid_id = false;
                    for note in notes.clone() {
                        if note.id == n as u64 {
                            notes = vec![note];
                            valid_id = true;
                        }
                    }
                    if !valid_id {
                        eprintln!("Invalid input");
                        exit(exitcode::DATAERR);
                    }
                }

                let note = &notes[0];

                println!("{}", note);
                print!("Do you want to remove this note? [y/n] ");
                io::stdout().flush().unwrap();
                let mut answer = String::new();
                io::stdin().read_line(&mut answer).unwrap();
                if answer.trim() != "y" {
                    println!("Not removing");
                    exit(exitcode::OK);
                }

                match note.remove(&conn) {
                    Ok(_) => println!("{}\nremoved successfully", note),
                    Err(_) => {
                        eprintln!("Error while removing {}", note);
                        exit(exitcode::DATAERR);
                    }
                }
            }
        },
        Commands::List(list) => match list.entity {
            ListEntity::People { name } => {
                let people: Vec<Person>;
                if let Some(name) = name {
                    people = match Person::get_by_name(&conn, Some(name), None) {
                        Ok(people) => people,
                        Err(e) => {
                            eprintln!("Error while fecthing person: {:#?}", e);
                            exit(exitcode::DATAERR);
                        }
                    }
                } else {
                    people = match Person::get_all(&conn) {
                        Ok(people) => people,
                        Err(e) => {
                            eprintln!("Error while fecthing person: {:#?}", e);
                            exit(exitcode::DATAERR);
                        }
                    };
                }

                for person in people.iter() {
                    println!("{}", person);
                }
            }
            ListEntity::Activities {} => {
                let activities = Activity::get_all(&conn);
                println!("listing activities: {:#?}", activities);
            }
            ListEntity::Reminders { include_past } => {
                let reminders = Reminder::get_all(&conn, include_past);
                if let Ok(reminder) = reminders {
                    println!("{:#?}", reminder);
                }
            }
            ListEntity::Notes {} => {
                let notes = Note::get_all(&conn);
                println!("listing notes: {:#?}", notes);
            }
            ListEntity::Events { days } => {
                let mut events = match Event::get_all(&conn, days) {
                    Ok(events) => events,
                    Err(e) => {
                        eprintln!("Error while fetching events: {:#?}", e);
                        exit(exitcode::DATAERR);
                    }
                };

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
            let events = match Event::get_all(&conn, 0) {
                Ok(events) => events,
                Err(e) => {
                    eprintln!("Error while fetching events: {:#?}", e);
                    exit(exitcode::DATAERR);
                }
            };
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
                        let dtstart = format!("{}", event.date.format("%Y%m%d"));
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
                        let dtdue = format!("{}T090000", event.date.format("%Y%m%d"));
                        todo.push(Summary::new(reminder.name));
                        todo.push(Comment::new(
                            reminder
                                .description
                                .unwrap_or_else(|| String::from("[Empty]")),
                        ));
                        todo.push(Status::needs_action());
                        todo.push(Due::new(dtdue));
                        calendar.add_todo(todo);
                    }
                }
            }
            match calendar.save_file("data/calendar.ics") {
                Ok(_) => println!("Saved to data/calendar.ics"),
                Err(e) => {
                    eprintln!("Error while saving to data/calendar.ics: {:#?}", e);
                    exit(exitcode::SOFTWARE);
                }
            };
        }
    }
}
