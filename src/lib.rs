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

pub enum Entities {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
    Note(Note),
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

        let rows = stmt
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

    // TODO might be a good idea to edit activities, reminders and notes vectors
    pub fn update(
        &mut self,
        name: Option<String>,
        birthday: Option<String>,
        contact_info: Option<String>,
    ) -> &Self {
        // TODO clean up duplication between this and main.rs
        if let Some(name) = name {
            self.name = name;
        }
        if let Some(birthday) = birthday {
            let birthday_obj: Option<NaiveDate>;
            // TODO proper error handling and messaging
            match crate::helpers::parse_from_str_ymd(&birthday) {
                Ok(date) => birthday_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&birthday) {
                    Ok(date) => birthday_obj = Some(date),
                    Err(error) => panic!("Error parsing birthday: {}", error),
                },
            }
            self.birthday = birthday_obj;
        }

        let contact_info_split: Vec<String>;
        let mut contact_info_type: Option<ContactInfoType> = None;
        // TODO allow for multiple contact info on creation
        match contact_info {
            Some(contact_info_str) => {
                contact_info_split = contact_info_str.split(":").map(|x| x.to_string()).collect()
            }
            None => contact_info_split = vec![],
        }

        if contact_info_split.len() > 0 {
            match contact_info_split[0].as_str() {
                "phone" => {
                    contact_info_type = Some(ContactInfoType::Phone(contact_info_split[1].clone()))
                }
                "whatsapp" => {
                    contact_info_type =
                        Some(ContactInfoType::WhatsApp(contact_info_split[1].clone()))
                }
                "email" => {
                    contact_info_type = Some(ContactInfoType::Email(contact_info_split[1].clone()))
                }
                // TODO proper error handling and messaging
                _ => panic!("Unknown contact info type"),
            }
        }

        let mut contact_info: Vec<ContactInfo> = Vec::new();
        if let Some(contact_info_type) = contact_info_type {
            contact_info.push(ContactInfo::new(0, self.id, contact_info_type));
        }

        self.contact_info = contact_info;

        self
    }
}

impl crate::db::db_interface::DbOperations for Person {
    fn add(
        &self,
        conn: &Connection,
    ) -> Result<&Person, crate::db::db_interface::DbOperationsError> {
        let mut error = false;
        let mut stmt = conn
            .prepare("SELECT id FROM people WHERE name = ?")
            .unwrap();
        let mut rows = stmt.query(params![self.name]).unwrap();
        let mut ids: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            ids.push(row.get(0).unwrap());
        }

        if ids.len() > 0 {
            return Err(crate::db::db_interface::DbOperationsError::DuplicateEntry);
        }

        // TODO make all db operations atomic
        let birthday_str = match self.birthday {
            Some(birthday) => birthday.to_string(),
            None => "".to_string(),
        };

        match conn.execute(
            "INSERT INTO people (name, birthday, deleted) VALUES (?1, ?2, FALSE)",
            params![self.name, birthday_str],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }
        let id = conn.last_insert_rowid();

        self.contact_info.iter().for_each(|contact_info| {
            let (ci_type, ci_value): (String, &str) = match &contact_info.contact_info_type {
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
                    contact_info_details,
                    deleted
                )
                    VALUES (?1, ?2, ?3, FALSE)",
                params![id, types[0], ci_value],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                // FIXME extract this to a separate function to leverage FromIterator Results
                Err(_) => error = true,
            }
        });

        if error {
            return Err(crate::db::db_interface::DbOperationsError::GenericError);
        }
        Ok(self)
    }

    fn remove(
        &self,
        conn: &Connection,
    ) -> Result<&Person, crate::db::db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE 
                    people 
                SET
                    deleted = TRUE
                WHERE
                    id = ?1",
            [self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn save(
        &self,
        conn: &Connection,
    ) -> Result<&Person, crate::db::db_interface::DbOperationsError> {
        let birthday_str = match self.birthday {
            Some(birthday) => birthday.to_string(),
            None => "".to_string(),
        };

        match conn.execute(
            "UPDATE
                people
            SET
                name = ?1,
                birthday = ?2
            WHERE
                id = ?3",
            params![self.name, birthday_str, self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
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
                .prepare("SELECT id FROM contact_info_types WHERE type = ?")
                .unwrap();
            let mut rows = stmt.query(params![ci_type]).unwrap();
            let mut types: Vec<u32> = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                types.push(row.get(0).unwrap());
            }

            let mut stmt = conn
                .prepare("SELECT id FROM contact_info WHERE person_id = ?")
                .unwrap();
            let mut rows = stmt.query(params![self.id]).unwrap();
            let mut ci_ids: Vec<u32> = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                ci_ids.push(row.get(0).unwrap());
            }

            match conn.execute(
                "UPDATE
                    contact_info 
                SET
                    person_id = ?1,
                    contact_info_type_id = ?2,
                    contact_info_details = ?3
                WHERE
                    id = ?4",
                params![self.id, types[0], ci_value, ci_ids[0]],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
            }
        }

        Ok(self)
    }

    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
        let mut stmt = conn
            .prepare("SELECT * FROM people WHERE id = ?1")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![id]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let person_id = row.get(0).unwrap();
                    Some(Entities::Person(Person {
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
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
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

    pub fn get_all(conn: &Connection) -> Vec<Activity> {
        let mut stmt = conn
            .prepare("SELECT * FROM activities")
            .expect("Invalid SQL statement");

        let rows = stmt
            .query_map([], |row| {
                let activity_id = row.get(0).unwrap();
                Ok(Activity {
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
            })
            .unwrap();

        let mut activities = Vec::new();

        for activity in rows.into_iter() {
            activities.push(activity.unwrap());
        }

        activities
    }

    // TODO might be a good idea to edit people
    pub fn update(
        &mut self,
        name: Option<String>,
        activity_type: Option<String>,
        date: Option<String>,
        content: Option<String>,
    ) -> &Self {
        // TODO clean up duplication between this and main.rs
        if let Some(name) = name {
            self.name = name;
        }

        if let Some(activity_type) = activity_type {
            let activity_type = match activity_type.as_str() {
                "phone" => ActivityType::Phone,
                "in_person" => ActivityType::InPerson,
                "online" => ActivityType::Online,
                // TODO proper error handling and messaging
                _ => panic!("Unknown activity type"),
            };

            self.activity_type = activity_type;
        }

        if let Some(date) = date {
            let date_obj: Option<NaiveDate>;
            // TODO proper error handling and messaging
            match crate::helpers::parse_from_str_ymd(&date) {
                Ok(date) => date_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&date) {
                    Ok(date) => date_obj = Some(date),
                    Err(error) => panic!("Error parsing date: {}", error),
                },
            }
            self.date = date_obj.unwrap();
        }

        if let Some(content) = content {
            self.content = content;
        }

        self
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
                activities (name, type, date, content, deleted)
                VALUES (?1, ?2, ?3, ?4, FALSE)
            ",
            params![self.name, types[0], date_str, self.content],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        let id = conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_activities (
                    person_id, 
                    activity_id,
                    deleted
                )
                    VALUES (?1, ?2, FALSE)",
                params![person.id, id],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
            }
        }

        Ok(self)
    }

    fn remove(&self, conn: &crate::Connection) -> Result<&Self, db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE 
                    activities 
                SET
                    deleted = TRUE
                WHERE
                    id = ?1",
            [self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn save(
        &self,
        conn: &Connection,
    ) -> Result<&Activity, crate::db::db_interface::DbOperationsError> {
        let activity_type_str = self.activity_type.as_ref();

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
            "UPDATE
                activities
            SET
                name = ?1,
                type = ?2,
                date = ?3,
                content = ?4
            WHERE
                id = ?5",
            params![
                self.name,
                types[0],
                self.date.to_string(),
                self.content,
                self.id,
            ],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }
    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
        let mut stmt = conn
            .prepare("SELECT * FROM activities WHERE id = ?1")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![id]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let activity_id = row.get(0).unwrap();
                    Some(Entities::Activity(Activity {
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
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
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
        let sql: String;
        let base_sql = "SELECT * FROM reminders";
        if include_past {
            sql = format!("{}", base_sql);
        } else {
            sql = format!("{} WHERE date > DATE()", base_sql);
        }

        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");
        let rows = stmt
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

    pub fn update(
        &mut self,
        name: Option<String>,
        date: Option<String>,
        description: Option<String>,
        recurring: Option<String>,
    ) -> &Self {
        if let Some(name) = name {
            self.name = name;
        }

        if let Some(date) = date {
            let date_obj: Option<NaiveDate>;
            // TODO proper error handling and messaging
            match crate::helpers::parse_from_str_ymd(&date) {
                Ok(date) => date_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&date) {
                    Ok(date) => date_obj = Some(date),
                    Err(error) => panic!("Error parsing date: {}", error),
                },
            }
            self.date = date_obj.unwrap();
        }

        // TODO we need a way to unset description
        if let Some(description) = description {
            self.description = Some(description);
        }

        // TODO remove duplication between here and main.rs
        let recurring_type = match recurring {
            Some(recurring_type_str) => match recurring_type_str.as_str() {
                "daily" => Some(RecurringType::Daily),
                "weekly" => Some(RecurringType::Weekly),
                "fortnightly" => Some(RecurringType::Fortnightly),
                "monthly" => Some(RecurringType::Monthly),
                "quarterly" => Some(RecurringType::Quarterly),
                "biannual" => Some(RecurringType::Biannual),
                "yearly" => Some(RecurringType::Yearly),
                _ => panic!("Unknown recurring pattern"),
            },
            None => None,
        };

        if let Some(recurring_type) = recurring_type {
            self.recurring = Some(recurring_type);
        }

        self
    }
}

impl crate::db::db_interface::DbOperations for Reminder {
    fn add(
        &self,
        conn: &Connection,
    ) -> Result<&Reminder, crate::db::db_interface::DbOperationsError> {
        let mut stmt = conn
            .prepare("SELECT id FROM reminders WHERE name = ?")
            .unwrap();
        let mut rows = stmt.query(params![self.name]).unwrap();
        let mut ids: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            ids.push(row.get(0).unwrap());
        }

        if ids.len() > 0 {
            return Err(crate::db::db_interface::DbOperationsError::DuplicateEntry);
        }

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
                reminders (name, date, recurring, description, deleted)
                VALUES (?1, ?2, ?3, ?4, FALSE)
            ",
            params![self.name, date_str, types[0], self.description],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        let id = conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_reminders (
                    person_id, 
                    reminder_id,
                    deleted
                )
                    VALUES (?1, ?2, FALSE)",
                params![person.id, id],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
            }
        }

        Ok(self)
    }

    fn remove(&self, conn: &crate::Connection) -> Result<&Self, db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE 
                    reminders 
                SET
                    deleted = TRUE
                WHERE
                    id = ?1",
            [self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn save(
        &self,
        conn: &Connection,
    ) -> Result<&Reminder, crate::db::db_interface::DbOperationsError> {
        // TODO allow for changing people
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
            "UPDATE
                reminders 
            SET
                name = ?1,
                date = ?2,
                recurring = ?3,
                description = ?4
            WHERE
                id = ?5
            ",
            params![self.name, date_str, types[0], self.description, self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
        let mut stmt = conn
            .prepare("SELECT * FROM reminders WHERE id = ?1")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![id]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let reminder_id = row.get(0).unwrap();
                    Some(Entities::Reminder(Reminder {
                        id: reminder_id,
                        name: row.get(1).unwrap(),
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        description: row.get(3).unwrap(),
                        recurring: crate::RecurringType::get_by_id(&conn, row.get(4).unwrap()),
                        people: crate::db::db_helpers::get_people_by_reminder(&conn, reminder_id),
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
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
    pub fn new(id: u64, person_id: u64, contact_info_type: ContactInfoType) -> ContactInfo {
        let details = match contact_info_type {
            ContactInfoType::Phone(ref value) => value.to_string(),
            ContactInfoType::WhatsApp(ref value) => value.to_string(),
            ContactInfoType::Email(ref value) => value.to_string(),
        };
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

    pub fn get_all(conn: &Connection) -> Vec<Note> {
        let mut stmt = conn
            .prepare("SELECT * FROM notes")
            .expect("Invalid SQL statement");

        let rows = stmt
            .query_map([], |row| {
                let note_id = row.get(0).unwrap();
                Ok(Note {
                    id: note_id,
                    date: crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(1).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    content: row.get(2).unwrap(),
                    people: crate::db::db_helpers::get_people_by_note(&conn, note_id),
                })
            })
            .unwrap();

        let mut notes = Vec::new();

        for note in rows.into_iter() {
            notes.push(note.unwrap());
        }

        notes
    }

    pub fn update(&mut self, date: Option<String>, content: Option<String>) -> &Self {
        if let Some(date) = date {
            let mut date_obj: Option<NaiveDate>;
            // TODO proper error handling and messaging
            match crate::helpers::parse_from_str_ymd(&date) {
                Ok(date) => date_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&date) {
                    Ok(date) => date_obj = Some(date),
                    Err(error) => panic!("Error parsing date: {}", error),
                },
            }
            self.date = date_obj.unwrap();
        }

        if let Some(content) = content {
            self.content = content;
        }

        self
    }
}

impl crate::db::db_interface::DbOperations for Note {
    fn add(&self, conn: &Connection) -> Result<&Note, crate::db::db_interface::DbOperationsError> {
        let date_str = self.date.to_string();

        match conn.execute(
            "INSERT INTO 
                notes (date, content, deleted)
                VALUES (?1, ?2, FALSE)
            ",
            params![date_str, self.content],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        let id = &conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_notes (
                    person_id, 
                    note_id,
                    deleted
                )
                    VALUES (?1, ?2, FALSE)",
                params![person.id, id],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
            }
        }

        Ok(self)
    }

    fn remove(&self, conn: &crate::Connection) -> Result<&Self, db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE 
                    notes 
                SET
                    deleted = TRUE
                WHERE
                    id = ?1",
            [self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn save(&self, conn: &Connection) -> Result<&Note, crate::db::db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE
                notes
            SET
                date = ?1,
                content = ?2
            WHERE
                id = ?3",
            params![self.date.to_string(), self.content, self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
        let mut stmt = conn
            .prepare("SELECT * FROM notes WHERE id = ?1")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![id]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let note_id = row.get(0).unwrap();
                    Some(Entities::Note(Note {
                        id: note_id,
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(1).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        content: row.get(2).unwrap(),
                        people: crate::db::db_helpers::get_people_by_note(&conn, note_id),
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }
}
