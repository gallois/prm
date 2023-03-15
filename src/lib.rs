use chrono::prelude::*;
use rusqlite::{params, params_from_iter, Connection};
use std::convert::AsRef;
use strum_macros::AsRefStr;

#[derive(Debug)]
pub struct Person {
    id: u64,
    name: String,
    birthday: Option<NaiveDate>,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
    notes: Vec<Notes>,
}

impl Person {
    // TODO create a macro for generating all these `new` functions
    pub fn new(
        id: u64,
        name: String,
        birthday: Option<NaiveDate>,
        contact_info: Vec<ContactInfo>,
    ) -> Person {
        Person {
            id,
            name,
            birthday,
            contact_info,
            activities: vec![],
            reminders: vec![],
            notes: vec![],
        }
    }

    pub fn get_by_name(conn: &Connection, name: &str) -> Option<Person> {
        let mut stmt = conn
            .prepare("SELECT * FROM people WHERE name = ?1 COLLATE NOCASE")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![name]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => Some(Person {
                    id: row.get(0).unwrap(),
                    name: row.get(1).unwrap(),
                    birthday: None,
                    contact_info: vec![],
                    activities: vec![],
                    reminders: vec![],
                    notes: vec![],
                }),
                None => return None,
            },
            Err(_) => return None,
        }
    }

    pub fn get_by_names(conn: &Connection, names: Vec<String>) -> Vec<Person> {
        if names.is_empty() {
            return vec![];
        }

        let vars = repeat_vars(names.len());
        let sql = format!(
            "SELECT * FROM people WHERE name in ({}) COLLATE NOCASE",
            vars
        );

        let mut people = vec![];
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");
        let rows: _ = stmt
            .query_map(params_from_iter(names.iter()), |row| {
                Ok(Person::new(
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    Some(
                        NaiveDate::parse_from_str(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                            "%Y-%m-%d",
                        )
                        .unwrap_or_default(),
                    ),
                    // Some(NaiveDate::parse_from_str(row.get(2).unwrap(), "%Y-%m-%d").unwrap()),
                    vec![],
                ))
            })
            .unwrap();

        for person in rows.into_iter() {
            people.push(person.unwrap());
        }

        people
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

        if self.contact_info.len() > 0 {
            let (ci_type, ci_value): (String, &str) = match &self.contact_info[0].contact_info_type
            {
                ContactInfoType::Phone(value) => (
                    ContactInfoType::Phone(value.clone()).as_ref().to_owned(),
                    value.as_ref(),
                ),
                ContactInfoType::WhatsApp(value) => (
                    ContactInfoType::WhatsApp(value.clone()).as_ref().to_owned(),
                    value.as_ref(),
                ),
                ContactInfoType::Email(value) => (
                    ContactInfoType::Email(value.clone()).as_ref().to_owned(),
                    value.as_ref(),
                ),
            };

            // TODO error handling
            let mut stmt = conn
                .prepare("SELECT id FROM people WHERE name = ?")
                .unwrap();
            let mut rows = stmt.query(params![self.name]).unwrap();
            let mut ids: Vec<u32> = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                ids.push(row.get(0).unwrap());
            }

            // TODO error handling
            let mut stmt = conn
                .prepare("SELECT id FROM contact_info_types WHERE type = ?")
                .unwrap();
            let mut rows = stmt.query(params![ci_type]).unwrap();
            let mut types: Vec<u32> = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                types.push(row.get(0).unwrap());
            }

            match conn.execute(
                "INSERT INTO contact_info (
                    person_id, 
                    contact_info_type_id, 
                    contact_info_details
                )
                    VALUES (?1, ?2, ?3)",
                params![ids[0], types[0], ci_value],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(DbOperationsError),
            }
        }

        Ok(self)
    }
}

#[derive(Debug)]
pub struct Activity {
    id: u64,
    name: String,
    activity_type: ActivityType,
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
}

impl Activity {
    pub fn new(
        id: u64,
        name: String,
        activity_type: ActivityType,
        date: NaiveDate,
        content: String,
        people: Vec<Person>,
    ) -> Activity {
        Activity {
            id,
            name,
            activity_type,
            date,
            content,
            people,
        }
    }
}

impl DbOperations for Activity {
    // TODO add people <> activity
    fn add(&self, conn: &Connection) -> Result<&Activity, DbOperationsError> {
        let activity_type_str = self.activity_type.as_ref();
        let date_str = self.date.to_string();

        // TODO error handling
        let mut stmt = conn
            .prepare("SELECT id FROM activity_types WHERE type = ?")
            .unwrap();
        let mut rows = stmt.query(params![activity_type_str]).unwrap();
        let mut types: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            types.push(row.get(0).unwrap());
        }

        match conn.execute(
            "INSERT INTO 
                activities (name, type, date, content)
                VALUES (?1, ?2, ?3, ?4)
            ",
            params![self.name, types[0], date_str, self.content],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError),
        }

        let mut stmt = conn
            .prepare(
                "SELECT 
                        * 
                    FROM 
                        activities 
                    WHERE 
                        name = ?1
                        AND type = ?2
                        AND date = ?3
                        AND content = ?4
                    ",
            )
            .unwrap();
        let mut rows = stmt
            .query(params![self.name, types[0], date_str, self.content])
            .unwrap();
        let mut ids: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            ids.push(row.get(0).unwrap());
        }

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_activities (
                    person_id, 
                    activity_id
                )
                    VALUES (?1, ?2)",
                params![person.id, ids[0]],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(DbOperationsError),
            }
        }

        Ok(self)
    }
}

#[derive(Debug, AsRefStr)]
pub enum ActivityType {
    Phone,
    InPerson,
    Online,
}

#[derive(Debug)]
pub struct Reminder {
    id: u64,
    name: String,
    date: NaiveDate,
    description: Option<String>,
    recurring: Option<RecurringType>,
    people: Vec<Person>,
}

impl Reminder {
    pub fn new(
        id: u64,
        name: String,
        date: NaiveDate,
        description: Option<String>,
        recurring: Option<RecurringType>,
        people: Vec<Person>,
    ) -> Reminder {
        Reminder {
            id,
            name,
            date,
            description,
            recurring,
            people,
        }
    }
}

impl DbOperations for Reminder {
    // TODO add people <> reminder
    fn add(&self, conn: &Connection) -> Result<&Reminder, DbOperationsError> {
        let recurring_str = match &self.recurring {
            Some(recurring_type) => recurring_type.as_ref(),
            None => "",
        };

        let date_str = self.date.to_string();

        // TODO error handling
        let mut stmt = conn
            .prepare("SELECT id FROM recurring_types WHERE type = ?")
            .unwrap();
        let mut rows = stmt.query(params![recurring_str]).unwrap();
        let mut types: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            types.push(row.get(0).unwrap());
        }

        match conn.execute(
            "INSERT INTO 
                reminders (name, date, recurring, description)
                VALUES (?1, ?2, ?3, ?4)
            ",
            params![self.name, date_str, recurring_str, self.description],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError),
        }
        Ok(self)
    }
}

#[derive(Debug, AsRefStr)]
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

#[derive(Debug, AsRefStr)]
pub enum ContactInfoType {
    Phone(String),
    WhatsApp(String),
    Email(String),
}

#[derive(Debug)]
pub struct Notes {
    id: u64,
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
}

impl Notes {
    pub fn new(id: u64, date: NaiveDate, content: String, people: Vec<Person>) -> Notes {
        Notes {
            id,
            date,
            content,
            people,
        }
    }
}

impl DbOperations for Notes {
    // TODO add people <> notes
    fn add(&self, conn: &Connection) -> Result<&Notes, DbOperationsError> {
        let date_str = self.date.to_string();

        match conn.execute(
            "INSERT INTO 
                notes (date, content)
                VALUES (?1, ?2)
            ",
            params![date_str, self.content],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError),
        }
        Ok(self)
    }
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
            // Improve message
            Ok(_) => println!("Database table created"),
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
            ('WhatsApp'),
            ('Email')
        ",
        "INSERT INTO activity_types (type)
         VALUES 
            ('Phone'),
            ('InPerson'),
            ('Online')
        ",
        "INSERT INTO recurring_types (type)
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
            // Improve message
            Ok(_) => println!("Database table populated"),
            Err(error) => {
                println!("Error populating database tables: {}", error);
                return Err(DbOperationsError);
            }
        }
    }
    Ok(())
}

// Helper function to return a comma-separated sequence of `?`.
// - `repeat_vars(0) => panic!(...)`
// - `repeat_vars(1) => "?"`
// - `repeat_vars(2) => "?,?"`
// - `repeat_vars(3) => "?,?,?"`
// - ...
fn repeat_vars(count: usize) -> String {
    assert_ne!(count, 0);
    let mut s = "?,".repeat(count);
    // Remove trailing comma
    s.pop();
    s
}
