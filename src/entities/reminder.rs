use chrono::prelude::*;
use rusqlite::{params, params_from_iter};
use std::{convert::AsRef, fmt, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::db::db_interface::DbOperationsError;
use crate::entities::person::Person;
use crate::entities::Entities;
use rusqlite::Connection;

use snafu::prelude::*;

// FIXME this is a duplication of what we have in `CliError` (src/cli/add.rs)
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum EntityError {
    #[snafu(display("Invalid date: {}", date))]
    DateParseError { date: String },
    #[snafu(display("Invalid recurring type: {}", recurring_type))]
    RecurringTypeParseError { recurring_type: String },
    #[snafu(display("Invalid record: {}", record))]
    RecordParseError { record: String },
}

pub static REMINDER_TEMPLATE: &str = "Name: {name}
Date: {date}
Recurring: {recurring_type}
Description: {description}
People: {people}
";
#[derive(Debug, Clone, PartialEq)]
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

    fn build_from_sql(
        conn: &Connection,
        id: Result<u64, rusqlite::Error>,
        name: Result<String, rusqlite::Error>,
        date: Result<String, rusqlite::Error>,
        description: Result<String, rusqlite::Error>,
        recurring_type_id: Result<u64, rusqlite::Error>,
    ) -> Result<Reminder, DbOperationsError> {
        let id = match id {
            Ok(reminder_id) => reminder_id,
            Err(e) => {
                return Err(DbOperationsError::RecordError {
                    sqlite_error: Some(e),
                    strum_error: None,
                })
            }
        };
        let name: String = match name {
            Ok(name) => name,
            Err(e) => {
                return Err(DbOperationsError::RecordError {
                    sqlite_error: Some(e),
                    strum_error: None,
                })
            }
        };
        let description: String = match description {
            Ok(description) => description,
            Err(e) => {
                return Err(DbOperationsError::RecordError {
                    sqlite_error: Some(e),
                    strum_error: None,
                })
            }
        };
        let recurring_type_id = match recurring_type_id {
            Ok(recurring_type_id) => recurring_type_id,
            Err(e) => {
                return Err(DbOperationsError::RecordError {
                    sqlite_error: Some(e),
                    strum_error: None,
                })
            }
        };
        let people = crate::db_helpers::get_people_by_reminder(&conn, id)?;
        let recurring_type = match RecurringType::get_by_id(&conn, recurring_type_id) {
            Ok(recurring_type) => match recurring_type {
                Some(recurring_type) => recurring_type,
                None => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: None,
                        strum_error: None,
                    })
                }
            },
            Err(e) => return Err(e),
        };
        let date =
            crate::helpers::parse_from_str_ymd(String::from(date.unwrap_or_default()).as_str())
                .unwrap_or_default();
        Ok(Reminder {
            id,
            name,
            date,
            description: Some(description),
            recurring: recurring_type,
            people,
        })
    }

    pub fn get(
        conn: &Connection,
        name: Option<String>,
        person: Option<String>,
    ) -> Result<Vec<Reminder>, DbOperationsError> {
        let mut reminders: Vec<Reminder> = vec![];

        match name {
            Some(name) => {
                let mut stmt = match conn
                    .prepare("SELECT * FROM reminders WHERE name = ?1 COLLATE NOCASE")
                {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };
                let mut rows = match stmt.query(params![name]) {
                    Ok(rows) => rows,
                    Err(_) => return Err(DbOperationsError::QueryError),
                };
                loop {
                    match rows.next() {
                        Ok(row) => match row {
                            Some(row) => {
                                let reminder = Self::build_from_sql(
                                    conn,
                                    row.get(0),
                                    row.get(1),
                                    row.get(2),
                                    row.get(3),
                                    row.get(4),
                                )?;
                                if let Some(person) = person.clone() {
                                    let people_name: Vec<String> =
                                        reminder.people.iter().map(|p| p.name.to_owned()).collect();
                                    if people_name.contains(&person) {
                                        reminders.push(reminder);
                                    }
                                } else {
                                    reminders.push(reminder);
                                }
                            }
                            None => return Ok(reminders),
                        },
                        Err(_) => return Err(DbOperationsError::GenericError),
                    }
                }
            }
            None => (),
        }
        match person {
            Some(person) => {
                let mut stmt = match conn
                    .prepare("SELECT id FROM people WHERE name = ?1 COLLATE NOCASE")
                {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };
                let mut rows = match stmt.query(params![person]) {
                    Ok(rows) => rows,
                    Err(_) => return Err(DbOperationsError::QueryError),
                };
                let person_id: u64;
                match rows.next() {
                    Ok(row) => match row {
                        Some(row) => {
                            person_id = match row.get(0) {
                                Ok(person_id) => person_id,
                                Err(e) => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: Some(e),
                                        strum_error: None,
                                    })
                                }
                            };
                            // TODO extract to a separate function
                            let mut reminder_ids: Vec<u8> = vec![];
                            let mut stmt = match conn.prepare(
                                "SELECT reminder_id FROM people_reminders WHERE person_id = ?1 COLLATE NOCASE",
                            ) {
                                Ok(stmt) => stmt,
                                Err(e) => {
                                    return Err(DbOperationsError::InvalidStatement {
                                        sqlite_error: e,
                                    })
                                }
                            };
                            let mut rows = match stmt.query(params![person_id]) {
                                Ok(rows) => rows,
                                Err(_) => return Err(DbOperationsError::QueryError),
                            };

                            loop {
                                match rows.next() {
                                    Ok(row) => match row {
                                        Some(row) => {
                                            match row.get(0) {
                                                Ok(id) => reminder_ids.push(id),
                                                Err(e) => {
                                                    return Err(DbOperationsError::RecordError {
                                                        sqlite_error: Some(e),
                                                        strum_error: None,
                                                    })
                                                }
                                            };
                                        }
                                        None => break,
                                    },
                                    Err(e) => {
                                        return Err(DbOperationsError::RecordError {
                                            sqlite_error: Some(e),
                                            strum_error: None,
                                        })
                                    }
                                }
                            }

                            let vars = crate::helpers::repeat_vars(reminder_ids.len());
                            let sql = format!(
                                "SELECT * from reminders WHERE id IN ({}) AND deleted = FALSE",
                                vars
                            );
                            let mut stmt = match conn.prepare(&sql) {
                                Ok(stmt) => stmt,
                                Err(e) => {
                                    return Err(DbOperationsError::InvalidStatement {
                                        sqlite_error: e,
                                    })
                                }
                            };
                            let mut rows = match stmt.query(params_from_iter(reminder_ids.iter())) {
                                Ok(rows) => rows,
                                Err(_) => return Err(DbOperationsError::QueryError),
                            };

                            loop {
                                match rows.next() {
                                    Ok(row) => match row {
                                        Some(row) => {
                                            let reminder = Self::build_from_sql(
                                                conn,
                                                row.get(0),
                                                row.get(1),
                                                row.get(2),
                                                row.get(3),
                                                row.get(4),
                                            )?;
                                            reminders.push(reminder);
                                        }
                                        None => break,
                                    },
                                    Err(_) => return Err(DbOperationsError::GenericError),
                                }
                            }
                        }
                        None => (),
                    },
                    Err(_) => return Err(DbOperationsError::GenericError),
                }
            }
            None => (),
        }
        return Ok(reminders);
    }

    // TODO remove duplication between different entities
    pub fn get_by_name(
        conn: &Connection,
        name: &str,
    ) -> Result<Option<Reminder>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM reminders WHERE name = ?1 COLLATE NOCASE")
        {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![name]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let reminder_id = match row.get(0) {
                        Ok(reminder_id) => reminder_id,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let name: String = match row.get(1) {
                        Ok(name) => name,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let description: Option<String> = match row.get(3) {
                        Ok(description) => description,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let recurring_type_id = match row.get(4) {
                        Ok(recurring_type_id) => recurring_type_id,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let people = crate::db_helpers::get_people_by_reminder(&conn, reminder_id)?;
                    let recurring_type = match RecurringType::get_by_id(&conn, recurring_type_id) {
                        Ok(recurring_type) => match recurring_type {
                            Some(recurring_type) => recurring_type,
                            None => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: None,
                                    strum_error: None,
                                })
                            }
                        },
                        Err(e) => return Err(e),
                    };
                    Ok(Some(Reminder {
                        id: reminder_id,
                        name,
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        description,
                        recurring: recurring_type,
                        people: people,
                    }))
                }
                None => return Ok(None),
            },
            Err(e) => {
                return Err(DbOperationsError::RecordError {
                    sqlite_error: Some(e),
                    strum_error: None,
                })
            }
        }
    }

    pub fn get_all(
        conn: &Connection,
        include_past: bool,
    ) -> Result<Vec<Reminder>, DbOperationsError> {
        let sql: String;
        let base_sql = "SELECT * FROM reminders";
        if include_past {
            sql = format!("{}", base_sql);
        } else {
            sql = format!("{} WHERE date > DATE()", base_sql);
        }

        let mut stmt = match conn.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let rows = match stmt.query_map([], |row| {
            let reminder_id = row.get(0)?;
            let people = match crate::db_helpers::get_people_by_reminder(&conn, reminder_id) {
                Ok(people) => people,
                Err(e) => {
                    let sqlite_error = match e {
                        DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                        other => panic!("Unexpected error type: {:#?}", other),
                    };
                    return Err(sqlite_error);
                }
            };
            let recurring_type_id: u64 = row.get(4)?;
            let recurring_type = match RecurringType::get_by_id(&conn, recurring_type_id) {
                Ok(recurring_type) => match recurring_type {
                    Some(recurring_type) => recurring_type,
                    None => panic!("Recurring Type cannot be None"),
                },
                Err(e) => {
                    let sqlite_error = match e {
                        DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                        other => panic!("Unexpected error type: {:#?}", other),
                    };
                    return Err(sqlite_error);
                }
            };
            Ok(Reminder {
                id: reminder_id,
                name: row.get(1)?,
                date: crate::helpers::parse_from_str_ymd(
                    String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                )
                .unwrap_or_default(),
                description: row.get(3)?,
                recurring: recurring_type,
                people: people,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut reminders = Vec::new();

        for reminder in rows.into_iter() {
            let reminder = match reminder {
                Ok(reminder) => reminder,
                Err(e) => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: Some(e),
                        strum_error: None,
                    })
                }
            };
            reminders.push(reminder);
        }

        Ok(reminders)
    }

    pub fn update(
        &mut self,
        conn: &Connection,
        name: Option<String>,
        date: Option<String>,
        description: Option<String>,
        recurring: Option<String>,
        people: Vec<String>,
    ) -> Result<&Self, EntityError> {
        if let Some(name) = name {
            self.name = name;
        }

        if let Some(date) = date {
            let date_obj: Option<NaiveDate>;
            match crate::helpers::parse_from_str_ymd(&date) {
                Ok(date) => date_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&date) {
                    Ok(date) => date_obj = Some(date),
                    Err(_) => {
                        return DateParseSnafu {
                            date: date.to_string(),
                        }
                        .fail()
                    }
                },
            }
            self.date = match date_obj {
                Some(date) => date,
                None => {
                    return DateParseSnafu {
                        date: date.to_string(),
                    }
                    .fail()
                }
            }
        }

        if let Some(description) = description {
            self.description = Some(description);
        } else {
            self.description = None;
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
                _ => {
                    return RecurringTypeParseSnafu {
                        recurring_type: recurring_type_str.to_string(),
                    }
                    .fail()
                }
            },
            None => Some(RecurringType::OneTime),
        };

        if let Some(recurring_type) = recurring_type {
            self.recurring = recurring_type;
        }

        let people = match Person::get_by_names(&conn, people) {
            Ok(people) => people,
            Err(_) => {
                return RecordParseSnafu {
                    record: "people".to_string(),
                }
                .fail()
            }
        };
        self.people = people;

        Ok(self)
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
            _ => error = true,
        });

        if error {
            return Err(crate::editor::ParseError::FormatError);
        }

        Ok((name, date, recurring_type, description, people))
    }
}

impl crate::db::db_interface::DbOperations for Reminder {
    fn add(&self, conn: &Connection) -> Result<&Reminder, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT id FROM reminders WHERE name = ?") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![self.name]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut ids: Vec<u32> = Vec::new();
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => ids.push(row),
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    },
                    None => break,
                },
                Err(e) => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: Some(e),
                        strum_error: None,
                    })
                }
            }
        }

        if ids.len() > 0 {
            return Err(DbOperationsError::DuplicateEntry);
        }

        let recurring_str = &self.recurring.as_ref();

        let date_str = self.date.to_string();

        let mut stmt = match conn.prepare("SELECT id FROM recurring_types WHERE type = ?") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![recurring_str]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut types: Vec<u32> = Vec::new();
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => types.push(row),
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    },
                    None => break,
                },
                Err(e) => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: Some(e),
                        strum_error: None,
                    })
                }
            }
        }

        let mut stmt = match conn.prepare(
            "INSERT INTO 
                reminders (name, date, recurring, description, deleted)
                VALUES (?1, ?2, ?3, ?4, FALSE)
            ",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        match stmt.execute(params![self.name, date_str, types[0], self.description]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        let id = conn.last_insert_rowid();

        for person in &self.people {
            let mut stmt = match conn.prepare(
                "INSERT INTO people_reminders (
                    person_id, 
                    reminder_id,
                    deleted
                )
                    VALUES (?1, ?2, FALSE)",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            match stmt.execute(params![person.id, id]) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(DbOperationsError::QueryError),
            }
        }

        Ok(self)
    }

    fn remove(&self, conn: &Connection) -> Result<&Self, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "UPDATE 
                    reminders 
                SET
                    deleted = TRUE
                WHERE
                    id = ?1",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        match stmt.execute([self.id]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        Ok(self)
    }

    fn save(&self, conn: &Connection) -> Result<&Reminder, DbOperationsError> {
        let recurring_str = &self.recurring.as_ref();

        let date_str = self.date.to_string();

        let mut stmt = match conn.prepare("SELECT id FROM recurring_types WHERE type = ?") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![recurring_str]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut types: Vec<u32> = Vec::new();
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => types.push(row),
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    },
                    None => break,
                },
                Err(e) => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: Some(e),
                        strum_error: None,
                    })
                }
            }
        }

        let mut stmt = match conn.prepare(
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
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        match stmt.execute(params![
            self.name,
            date_str,
            types[0],
            self.description,
            self.id
        ]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        // TODO allow for changing people
        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Result<Option<Entities>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM reminders WHERE id = ?1") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let reminder_id = match row.get(0) {
                        Ok(reminder_id) => reminder_id,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let name: String = match row.get(1) {
                        Ok(name) => name,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let people = crate::db_helpers::get_people_by_reminder(&conn, reminder_id)?;
                    let description: Option<String> = match row.get(4) {
                        Ok(description) => description,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let recurring_type_id = match row.get(4) {
                        Ok(recurring_type_id) => recurring_type_id,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let recurring_type = match RecurringType::get_by_id(&conn, recurring_type_id) {
                        Ok(recurring_type) => match recurring_type {
                            Some(recurring_type) => recurring_type,
                            None => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: None,
                                    strum_error: None,
                                })
                            }
                        },
                        Err(e) => return Err(e),
                    };
                    Ok(Some(Entities::Reminder(Reminder {
                        id: reminder_id,
                        name,
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        description,
                        recurring: recurring_type,
                        people: people,
                    })))
                }
                None => return Ok(None),
            },
            Err(e) => {
                return Err(DbOperationsError::RecordError {
                    sqlite_error: Some(e),
                    strum_error: None,
                })
            }
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

#[derive(Debug, AsRefStr, EnumString, Clone, PartialEq)]
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
    pub fn get_by_id(
        conn: &Connection,
        id: u64,
    ) -> Result<Option<RecurringType>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT type FROM recurring_types WHERE id = ?") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let recurring_type_str = match row.get::<usize, String>(0) {
                        Ok(recurring_type_str) => recurring_type_str,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let recurring_type = match RecurringType::from_str(recurring_type_str.as_str())
                    {
                        Ok(recurring_type) => recurring_type,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: None,
                                strum_error: Some(e),
                            })
                        }
                    };
                    Ok(Some(recurring_type))
                }
                None => Ok(None),
            },
            Err(e) => {
                return Err(DbOperationsError::RecordError {
                    sqlite_error: Some(e),
                    strum_error: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let id = 1;
        let name = String::from("I forgot");
        let date = crate::helpers::parse_from_str_ymd("2022-01-01").unwrap();
        let description = String::from("I don't remember");
        let recurring = RecurringType::Daily;
        let people: Vec<Person> = vec![];

        let reminder = Reminder::new(
            id,
            name.clone(),
            date,
            Some(description.clone()),
            recurring.clone(),
            people.clone(),
        );

        assert_eq!(
            Reminder {
                id,
                name,
                date,
                description: Some(description),
                recurring,
                people,
            },
            reminder
        );
    }
}
