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
    fn save(&self, conn: &Connection) -> Result<&Person, DbOperationsError> {
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
    fn save(&self, conn: &Connection) -> Result<&Self, DbOperationsError>
    where
        Self: Sized;
}
