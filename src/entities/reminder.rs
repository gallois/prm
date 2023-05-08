use chrono::prelude::*;
use rusqlite::{params, Connection};
use std::{convert::AsRef, fmt, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::entities::person::Person;
use crate::entities::Entities;

pub static REMINDER_TEMPLATE: &str = "Name: {name}
Date: {date}
Recurring: {recurring_type}
Description: {description}
People: {people}
";
#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: u64,
    pub name: String,
    pub date: NaiveDate,
    pub description: Option<String>,
    pub recurring: RecurringType,
    pub people: Vec<Person>,
}

impl Reminder {
    pub fn new(
        id: u64,
        name: String,
        date: NaiveDate,
        description: Option<String>,
        recurring: RecurringType,
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
                        recurring: RecurringType::get_by_id(&conn, row.get(4).unwrap()).unwrap(),
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
                    recurring: RecurringType::get_by_id(&conn, row.get(4).unwrap()).unwrap(),
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
        conn: &Connection,
        name: Option<String>,
        date: Option<String>,
        description: Option<String>,
        recurring: Option<String>,
        people: Vec<String>,
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
                "onetime" => Some(RecurringType::OneTime),
                _ => panic!("Unknown recurring pattern"),
            },
            None => Some(RecurringType::OneTime),
        };

        if let Some(recurring_type) = recurring_type {
            self.recurring = recurring_type;
        }

        let people = Person::get_by_names(&conn, people);
        self.people = people;

        self
    }

    pub fn parse_from_editor(
        content: &str,
    ) -> Result<
        (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Vec<String>,
        ),
        crate::editor::ParseError,
    > {
        let mut error = false;
        let mut name: String = String::new();
        let mut date: Option<String> = None;
        let mut recurring_type: Option<String> = None;
        let mut description: Option<String> = None;
        let mut people = Vec::new();

        let name_prefix = "Name: ";
        let date_prefix = "Date: ";
        let recurring_type_prefix = "Recurring: ";
        let description_prefix = "Description: ";
        let people_prefix = "People: ";

        content.lines().for_each(|line| match line {
            s if s.starts_with(name_prefix) => {
                name = s.trim_start_matches(name_prefix).to_string();
            }
            s if s.starts_with(date_prefix) => {
                date = Some(s.trim_start_matches(date_prefix).to_string());
            }
            s if s.starts_with(recurring_type_prefix) => {
                recurring_type = Some(s.trim_start_matches(recurring_type_prefix).to_string());
            }
            s if s.starts_with(description_prefix) => {
                description = Some(s.trim_start_matches(description_prefix).to_string());
            }
            s if s.starts_with(people_prefix) => {
                let people_str = s.trim_start_matches(people_prefix);
                people = people_str.split(",").map(|x| x.to_string()).collect();
            }
            // FIXME
            _ => error = true,
        });

        if error {
            return Err(crate::editor::ParseError::FormatError);
        }

        Ok((name, date, recurring_type, description, people))
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

        let recurring_str = &self.recurring.as_ref();

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

    fn remove(
        &self,
        conn: &Connection,
    ) -> Result<&Self, crate::db::db_interface::DbOperationsError> {
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
        let recurring_str = &self.recurring.as_ref();

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

        // TODO allow for changing people
        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Option<Entities> {
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
                        recurring: RecurringType::get_by_id(&conn, row.get(4).unwrap()).unwrap(),
                        people: crate::db::db_helpers::get_people_by_reminder(&conn, reminder_id),
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }
}

impl fmt::Display for Reminder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description_str = match &self.description {
            Some(description) => description.as_ref(),
            None => "",
        };
        let recurring_type_str = &self.recurring.as_ref();
        let mut people_str = String::new();
        for person in self.people.iter() {
            people_str.push_str("\n\t");
            people_str.push_str(format!("name: {}\n\t", person.name).as_ref());
        }
        write!(
            f,
            "reminder id: {}\nname: {}\ndate: {}\ndescription: {}\nrecurring type: {}\npeople:{}\n",
            &self.id,
            &self.name,
            &self.date.to_string(),
            description_str,
            recurring_type_str,
            people_str
        )
    }
}

#[derive(Debug, AsRefStr, EnumString, Clone)]
pub enum RecurringType {
    OneTime,
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Biannual,
    Yearly,
}

impl RecurringType {
    pub fn get_by_id(conn: &Connection, id: u64) -> Option<RecurringType> {
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