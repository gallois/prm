pub mod db;

pub use crate::db::{db_helpers, db_interface};

use chrono::prelude::*;
use rusqlite::{params, params_from_iter, Connection};
use std::{convert::AsRef, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

pub mod helpers {
    // Helper function to return a comma-separated sequence of `?`.
    // - `repeat_vars(0) => panic!(...)`
    // - `repeat_vars(1) => "?"`
    // - `repeat_vars(2) => "?,?"`
    // - `repeat_vars(3) => "?,?,?"`
    // - ...
    pub fn repeat_vars(count: usize) -> String {
        assert_ne!(count, 0);
        let mut s = "?,".repeat(count);
        // Remove trailing comma
        s.pop();
        s
    }

    pub fn parse_from_str_ymd(date: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
    }

    pub fn parse_from_str_md(date: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
        parse_from_str_ymd(format!("1-{}", date).as_ref())
    }
}

#[derive(Debug)]
pub struct Person {
    id: u64,
    name: String,
    birthday: Option<NaiveDate>,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
    notes: Vec<Note>,
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
                Some(row) => {
                    let person_id = row.get(0).unwrap();
                    Some(Person {
                        id: person_id,
                        name: row.get(1).unwrap(),
                        birthday: Some(
                            crate::helpers::parse_from_str_ymd(
                                String::from(row.get::<usize, String>(2).unwrap_or_default())
                                    .as_str(),
                            )
                            .unwrap_or_default(),
                        ),
                        contact_info: crate::db::db_helpers::get_contact_info_by_person(
                            &conn, person_id,
                        ),
                        activities: crate::db::db_helpers::get_activities_by_person(
                            &conn, person_id,
                        ),
                        reminders: crate::db::db_helpers::get_reminders_by_person(&conn, person_id),
                        notes: crate::db::db_helpers::get_notes_by_person(&conn, person_id),
                    })
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }

    pub fn get_by_names(conn: &Connection, names: Vec<String>) -> Vec<Person> {
        if names.is_empty() {
            return vec![];
        }

        let vars = crate::helpers::repeat_vars(names.len());
        let sql = format!(
            "SELECT * FROM people WHERE name IN ({}) COLLATE NOCASE",
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
                        crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    vec![],
                ))
            })
            .unwrap();

        for person in rows.into_iter() {
            people.push(person.unwrap());
        }

        people
    }

    pub fn get_all(conn: &Connection) -> Vec<Person> {
        let mut stmt = conn
            .prepare("SELECT * FROM people")
            .expect("Invalid SQL statement");

        let mut rows = stmt
            .query_map([], |row| {
                let person_id = row.get(0).unwrap();
                Ok(Person {
                    id: person_id,
                    name: row.get(1).unwrap(),
                    birthday: Some(
                        crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    contact_info: crate::db::db_helpers::get_contact_info_by_person(
                        &conn, person_id,
                    ),
                    activities: crate::db::db_helpers::get_activities_by_person(&conn, person_id),
                    reminders: crate::db::db_helpers::get_reminders_by_person(&conn, person_id),
                    notes: crate::db::db_helpers::get_notes_by_person(&conn, person_id),
                })
            })
            .unwrap();

        let mut people = Vec::new();

        for person in rows.into_iter() {
            people.push(person.unwrap());
        }

        people
    }
}

impl crate::db::db_interface::DbOperations for Person {
    fn add(
        &self,
        conn: &Connection,
    ) -> Result<&Person, crate::db::db_interface::DbOperationsError> {
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
            Err(_) => return Err(crate::db::db_interface::DbOperationsError),
        }
        let id = conn.last_insert_rowid();

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
                params![id, types[0], ci_value],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError),
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

    // TODO remove duplication between different entities
    pub fn get_by_name(conn: &Connection, name: &str) -> Option<Activity> {
        let mut stmt = conn
            .prepare("SELECT * FROM activities WHERE name = ?1 COLLATE NOCASE")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![name]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let activity_id = row.get(0).unwrap();
                    Some(Activity {
                        id: activity_id,
                        name: row.get(1).unwrap(),
                        activity_type: crate::ActivityType::get_by_id(&conn, row.get(2).unwrap())
                            .unwrap(),
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        content: row.get(4).unwrap(),
                        people: crate::db::db_helpers::get_people_by_activity(&conn, activity_id),
                    })
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }
}

impl crate::db::db_interface::DbOperations for Activity {
    fn add(
        &self,
        conn: &Connection,
    ) -> Result<&Activity, crate::db::db_interface::DbOperationsError> {
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
            Err(_) => return Err(crate::db::db_interface::DbOperationsError),
        }

        let id = conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_activities (
                    person_id, 
                    activity_id
                )
                    VALUES (?1, ?2)",
                params![person.id, id],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError),
            }
        }

        Ok(self)
    }
}

#[derive(Debug, AsRefStr, EnumString)]
pub enum ActivityType {
    Phone,
    InPerson,
    Online,
}

impl ActivityType {
    fn get_by_id(conn: &Connection, id: u64) -> Option<ActivityType> {
        let mut stmt = conn
            .prepare("SELECT type FROM activity_types WHERE id = ?")
            .unwrap();
        let mut rows = stmt.query(params![id]).unwrap();

        match rows.next() {
            Ok(row) => match row {
                Some(row) => Some(
                    ActivityType::from_str(row.get::<usize, String>(0).unwrap().as_str()).unwrap(),
                ),
                None => None,
            },
            Err(_) => None,
        }
    }
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

    // TODO remove duplication between different entities
    pub fn get_by_name(conn: &Connection, name: &str) -> Option<Reminder> {
        let mut stmt = conn
            .prepare("SELECT * FROM reminders WHERE name = ?1 COLLATE NOCASE")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![name]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let reminder_id = row.get(0).unwrap();
                    Some(Reminder {
                        id: reminder_id,
                        name: row.get(1).unwrap(),
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        description: row.get(3).unwrap(),
                        recurring: crate::RecurringType::get_by_id(&conn, row.get(4).unwrap()),
                        people: crate::db::db_helpers::get_people_by_reminder(&conn, reminder_id),
                    })
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }

    pub fn get_all(conn: &Connection, include_past: bool) -> Vec<Reminder> {
        let mut sql = String::from("");
        let base_sql = "SELECT * FROM reminders";
        if include_past {
            sql = format!("{}", base_sql);
        } else {
            sql = format!("{} WHERE date > DATE()", base_sql);
        }

        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");
        let mut rows = stmt
            .query_map([], |row| {
                let reminder_id = row.get(0).unwrap();
                Ok(Reminder {
                    id: reminder_id,
                    name: row.get(1).unwrap(),
                    date: crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    description: row.get(3).unwrap(),
                    recurring: crate::RecurringType::get_by_id(&conn, row.get(4).unwrap()),
                    people: crate::db::db_helpers::get_people_by_reminder(&conn, reminder_id),
                })
            })
            .unwrap();

        let mut reminders = Vec::new();

        for reminder in rows.into_iter() {
            reminders.push(reminder.unwrap());
        }

        reminders
    }
}

impl crate::db::db_interface::DbOperations for Reminder {
    fn add(
        &self,
        conn: &Connection,
    ) -> Result<&Reminder, crate::db::db_interface::DbOperationsError> {
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
            params![self.name, date_str, types[0], self.description],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError),
        }

        let id = conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_reminders (
                    person_id, 
                    reminder_id
                )
                    VALUES (?1, ?2)",
                params![person.id, id],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError),
            }
        }

        Ok(self)
    }
}

#[derive(Debug, AsRefStr, EnumString)]
pub enum RecurringType {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Biannual,
    Yearly,
}

impl RecurringType {
    fn get_by_id(conn: &Connection, id: u64) -> Option<RecurringType> {
        let mut stmt = conn
            .prepare("SELECT type FROM recurring_types WHERE id = ?")
            .unwrap();
        let mut rows = stmt.query(params![id]).unwrap();

        match rows.next() {
            Ok(row) => match row {
                Some(row) => Some(
                    RecurringType::from_str(row.get::<usize, String>(0).unwrap().as_str()).unwrap(),
                ),
                None => None,
            },
            Err(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct ContactInfo {
    id: u64,
    person_id: u64,
    pub contact_info_type: ContactInfoType,
    details: String,
}

impl ContactInfo {
    fn new(
        id: u64,
        person_id: u64,
        contact_info_type: ContactInfoType,
        details: String,
    ) -> ContactInfo {
        ContactInfo {
            id,
            person_id,
            contact_info_type,
            details,
        }
    }
}

#[derive(Debug, AsRefStr, EnumString)]
pub enum ContactInfoType {
    Phone(String),
    WhatsApp(String),
    Email(String),
}

impl ContactInfoType {
    fn get_by_id(conn: &Connection, id: u64) -> Option<ContactInfoType> {
        let mut stmt = conn
            .prepare("SELECT type FROM contact_info_types WHERE id = ?")
            .unwrap();
        let mut rows = stmt.query(params![id]).unwrap();

        match rows.next() {
            Ok(row) => match row {
                Some(row) => Some(
                    ContactInfoType::from_str(row.get::<usize, String>(0).unwrap().as_str())
                        .unwrap(),
                ),
                None => None,
            },
            Err(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct Note {
    id: u64,
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
}

impl Note {
    pub fn new(id: u64, date: NaiveDate, content: String, people: Vec<Person>) -> Note {
        Note {
            id,
            date,
            content,
            people,
        }
    }

    pub fn get_by_person(conn: &Connection, person: String) -> Vec<Note> {
        let person = crate::Person::get_by_name(&conn, &person);
        match person {
            Some(person) => person.notes,
            None => vec![],
        }
    }
}

impl crate::db::db_interface::DbOperations for Note {
    fn add(&self, conn: &Connection) -> Result<&Note, crate::db::db_interface::DbOperationsError> {
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
            Err(_) => return Err(crate::db::db_interface::DbOperationsError),
        }

        let id = &conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_notes (
                    person_id, 
                    note_id
                )
                    VALUES (?1, ?2)",
                params![person.id, id],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError),
            }
        }

        Ok(self)
    }
}
