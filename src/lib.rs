use chrono::prelude::*;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct Person {
    pub name: String,
    birthday: Option<NaiveDate>,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
    notes: Vec<Notes>,
}

impl Person {
    // TODO create a macro for generating all these `new` functions
    pub fn new(
        name: String,
        birthday: Option<NaiveDate>,
        contact_info: Vec<ContactInfo>,
    ) -> Person {
        Person {
            name,
            birthday,
            contact_info,
            activities: vec![],
            reminders: vec![],
            notes: vec![],
        }
    }
}

impl DbOperations for Person {
    fn add(&self, conn: &Connection) -> Result<&Person, DbOperationsError> {
        // TODO make all db operations atomic
        let birthday_str = match self.birthday {
            Some(birthday) => birthday.to_string(),
            None => "".to_string(),
        };
        match conn.execute(
            "INSERT INTO people (name, birthday) VALUES (?1, ?2)",
            params![self.name, birthday_str],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError),
        }
        Ok(self)
    }
}

#[derive(Debug)]
pub struct Activity {
    name: String,
    activity_type: ActivityType,
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
}

impl Activity {
    pub fn new(
        name: String,
        activity_type: ActivityType,
        date: NaiveDate,
        content: String,
        people: Vec<Person>,
    ) -> Activity {
        Activity {
            name,
            activity_type,
            date,
            content,
            people,
        }
    }
}

#[derive(Debug)]
pub enum ActivityType {
    Phone,
    InPerson,
    Online,
}

#[derive(Debug)]
pub struct Reminder {
    name: String,
    date: NaiveDate,
    description: Option<String>,
    recurring: Option<RecurringType>,
    people: Vec<Person>,
}

impl Reminder {
    pub fn new(
        name: String,
        date: NaiveDate,
        description: Option<String>,
        recurring: Option<RecurringType>,
        people: Vec<Person>,
    ) -> Reminder {
        Reminder {
            name,
            date,
            description,
            recurring,
            people,
        }
    }
}

#[derive(Debug)]
pub enum RecurringType {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Biannual,
    Yearly,
}

#[derive(Debug)]
pub struct ContactInfo {
    pub contact_info_type: ContactInfoType,
}

#[derive(Debug)]
pub enum ContactInfoType {
    Phone(String),
    WhatsApp(String),
    Email(String),
}

#[derive(Debug)]
pub struct Notes {
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
}

impl Notes {
    pub fn new(date: NaiveDate, content: String, people: Vec<Person>) -> Notes {
        Notes {
            date,
            content,
            people,
        }
    }
}

enum EntityType {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
    Notes(Notes),
}

pub struct DbOperationsError;

pub trait DbOperations {
    fn add(&self, conn: &Connection) -> Result<&Self, DbOperationsError>
    where
        Self: Sized;
}

pub fn init_db(conn: &Connection) -> Result<(), DbOperationsError> {
    let sql_create_statements = vec![
        "CREATE TABLE people (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            birthday TEXT
        );",
        "CREATE TABLE activities (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            type INTEGER NOT NULL,
            date TEXT NOT NULL,
            content TEXT
        );",
        "CREATE TABLE reminders (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            date TEXT NOT NULL,
            description TEXT,
            recurring INTEGER NOT NULL
        );",
        "CREATE TABLE notes (
            id INTEGER PRIMARY KEY, 
            date TEXT NOT NULL,
            content TEXT NOT NULL
        );",
        "CREATE TABLE contact_info (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            contact_info_type_id INTEGER NOT NULL,
            contact_info_details TEXT
        );",
        "CREATE TABLE contact_info_types (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL
        );",
        "CREATE TABLE people_activities (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            activity_id INTEGER NOT NULL
        );",
        "CREATE TABLE people_reminders (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            reminder_id INTEGER NOT NULL
        );",
        "CREATE TABLE people_notes (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            note_id INTEGER NOT NULL
        );",
        "CREATE TABLE activity_types (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL
        );",
        "CREATE TABLE recurring_types (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL
        );",
    ];
    for query in sql_create_statements {
        match conn.execute(query, ()) {
            Ok(_) => println!("Database tables created"),
            Err(error) => {
                println!("Error creating database tables: {}", error);
                return Err(DbOperationsError);
            }
        }
    }
    let sql_populate_statements = vec![
        "INSERT INTO contact_info_types (type) 
         VALUES 
            ('Phone'),
            ('Whatsapp'),
            ('Email')
        ",
        "INSERT INTO activity_types (type)
         VALUES 
            ('Phone'),
            ('InPerson'),
            ('Online')
        ",
        "INSERT INTO recurring_type (type)
         VALUES
            ('Daily'),
            ('Weekly'),
            ('Fortnightly'),
            ('Monthly'),
            ('Quarterly'),
            ('Biannual'),
            ('Yearly')
        ",
    ];
    for query in sql_populate_statements {
        match conn.execute(query, ()) {
            Ok(_) => println!("Database tables populated"),
            Err(error) => {
                println!("Error populating database tables: {}", error);
                return Err(DbOperationsError);
            }
        }
    }
    Ok(())
}
