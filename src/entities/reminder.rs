use chrono::prelude::*;
use rusqlite::params;
use std::{convert::AsRef, fmt, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::db::db_interface::DbOperationsError;
use crate::db_interface::DbOperations;
use crate::entities::person::Person;
use crate::entities::Entities;
use crate::{CliError, DateParseSnafu, RecordParseSnafu, RecurringTypeParseSnafu};
use rusqlite::Connection;

use super::Entity;

pub struct ParseReminderFromEditorData {
    pub name: String,
    pub date: Option<String>,
    pub recurring_type: Option<String>,
    pub description: Option<String>,
    pub people: Vec<String>,
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
impl Entity for Reminder {
    fn get_id(&self) -> u64 {
        self.id
    }
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

    pub fn build_from_sql(
        conn: &Connection,
        id: Result<u64, rusqlite::Error>,
        name: Result<String, rusqlite::Error>,
        date: Result<String, rusqlite::Error>,
        description: Result<Option<String>, rusqlite::Error>,
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
            Ok(description) => match description {
                Some(description) => description,
                None => "".to_string(),
            },
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
        let people = crate::db_helpers::people::get_by_reminder(conn, id)?;
        let recurring_type = match RecurringType::get_by_id(conn, recurring_type_id) {
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
        let date = crate::helpers::parse_from_str_ymd(date.unwrap_or_default().as_str())
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
        description: Option<String>,
    ) -> Result<Vec<Reminder>, DbOperationsError> {
        let mut reminders: Vec<Reminder> = vec![];

        if let Some(name) = name {
            reminders = crate::db::db_helpers::reminders::get_by_name(conn, &name, person.clone())?;
            return Ok(reminders);
        }
        if let Some(person) = person {
            reminders = crate::db::db_helpers::reminders::get_by_person(conn, person.clone())?;
            return Ok(reminders);
        }

        if let Some(description) = description {
            reminders = crate::db::db_helpers::reminders::get_by_description(conn, description)?;
        }
        Ok(reminders)
    }

    pub fn get_all_filtered(
        conn: &Connection,
        include_past: bool,
    ) -> Result<Vec<Reminder>, DbOperationsError> {
        let reminders = Reminder::get_all(conn)?;
        let filtered_reminders: Vec<Reminder> = reminders
            .iter()
            .map(|r| *r.to_owned())
            .filter(|r| include_past || r.date > chrono::Local::now().date_naive())
            .collect::<Vec<_>>();

        Ok(filtered_reminders)
    }

    pub fn update(
        &mut self,
        conn: &Connection,
        name: Option<String>,
        date: Option<String>,
        description: Option<String>,
        recurring: Option<String>,
        people: Vec<String>,
    ) -> Result<&Self, CliError> {
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

        let people = match crate::db::db_helpers::people::get_by_names(conn, people) {
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

    pub fn parse_from_editor(content: &str) -> Result<ParseReminderFromEditorData, CliError> {
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
                people = people_str.split(',').map(|x| x.to_string()).collect();
            }
            _ => error = true,
        });

        if error {
            return Err(CliError::FormatError);
        }

        Ok(ParseReminderFromEditorData {
            name,
            date,
            recurring_type,
            description,
            people,
        })
    }
}

impl crate::db::db_interface::DbOperations for Reminder {
    fn add(&self, conn: &Connection) -> Result<&Reminder, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT id FROM reminders WHERE name = ? AND deleted = 0")
        {
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

        if !ids.is_empty() {
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
        let mut stmt = match conn.prepare("SELECT * FROM reminders WHERE id = ?1 AND deleted = 0") {
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
                    let people = crate::db_helpers::people::get_by_reminder(conn, reminder_id)?;
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
                    let recurring_type = match RecurringType::get_by_id(conn, recurring_type_id) {
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
                            row.get::<usize, String>(2).unwrap_or_default().as_str(),
                        )
                        .unwrap_or_default(),
                        description,
                        recurring: recurring_type,
                        people,
                    })))
                }
                None => Ok(None),
            },
            Err(e) => Err(DbOperationsError::RecordError {
                sqlite_error: Some(e),
                strum_error: None,
            }),
        }
    }
    fn get_all(conn: &Connection) -> Result<Vec<Box<Self>>, DbOperationsError> {
        let sql = "SELECT * FROM reminders WHERE deleted = 0";
        let mut stmt = match conn.prepare(sql) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let rows = match stmt.query_map([], |row| {
            let reminder_id = row.get(0)?;
            let people = match crate::db_helpers::people::get_by_reminder(conn, reminder_id) {
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
            let recurring_type = match RecurringType::get_by_id(conn, recurring_type_id) {
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
                    row.get::<usize, String>(2).unwrap_or_default().as_str(),
                )
                .unwrap_or_default(),
                description: row.get(3)?,
                recurring: recurring_type,
                people,
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
            reminders.push(Box::new(reminder));
        }

        Ok(reminders)
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
            people_str.push_str(format!("name: {}", person.name).as_ref());
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
            Err(e) => Err(DbOperationsError::RecordError {
                sqlite_error: Some(e),
                strum_error: None,
            }),
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
