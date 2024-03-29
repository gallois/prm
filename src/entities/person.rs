use chrono::prelude::*;
use rusqlite::params;
use std::{convert::AsRef, fmt, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::db::db_interface::DbOperationsError;
use crate::db_interface::DbOperations;
use crate::entities::activity::Activity;
use crate::entities::note::Note;
use crate::entities::reminder::Reminder;
use crate::entities::Entities;
use crate::helpers::get_contact_info;
use crate::{BirthdayParseSnafu, CliError};
use rusqlite::Connection;

use super::Entity;

pub static PERSON_TEMPLATE: &str = "Name: {name}
Birthday: {birthday}
Contact Info: {contact_info}
Activities: {activities}
Reminders: {reminders}
";
#[derive(Debug, Clone, PartialEq)]
pub struct Person {
    pub id: u64,
    pub name: String,
    pub birthday: Option<NaiveDate>,
    pub contact_info: Vec<ContactInfo>,
    pub activities: Vec<Activity>,
    pub reminders: Vec<Reminder>,
    pub notes: Vec<Note>,
}

pub struct EditorData {
    pub name: String,
    pub birthday: Option<String>,
    pub contact_info: Vec<String>,
    pub activities: Vec<u64>,
    pub reminders: Vec<u64>,
}

impl Entity for Person {
    fn get_id(&self) -> u64 {
        self.id
    }
}

impl Person {
    // TODO create a macro for generating all these `new` functions
    pub fn new(
        id: u64,
        name: String,
        birthday: Option<NaiveDate>,
        contact_info: Vec<ContactInfo>,
        activities: Vec<Activity>,
        reminders: Vec<Reminder>,
    ) -> Person {
        Person {
            id,
            name,
            birthday,
            contact_info,
            activities,
            reminders,
            notes: vec![],
        }
    }

    // TODO might be a good idea to edit activities, reminders and notes vectors
    pub fn update(
        &mut self,
        name: String,
        birthday: Option<String>,
        contact_info: Option<String>,
        activities: Vec<Activity>,
        reminders: Vec<Reminder>,
    ) -> Result<&Self, CliError> {
        self.name = name;
        if let Some(birthday) = birthday {
            let birthday_obj: Option<NaiveDate>;
            match crate::helpers::parse_from_str_ymd(&birthday) {
                Ok(date) => birthday_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&birthday) {
                    Ok(date) => birthday_obj = Some(date),
                    Err(_) => return BirthdayParseSnafu { birthday }.fail(),
                },
            }
            self.birthday = birthday_obj;
        }

        let mut contact_info_splits: Vec<Vec<String>> = vec![];
        let mut contact_info_vec: Vec<ContactInfo> = Vec::new();
        match contact_info {
            Some(contact_info_vec) => {
                for contact_info_str in contact_info_vec.split(',') {
                    contact_info_splits
                        .push(contact_info_str.split(':').map(|x| x.to_string()).collect());
                }
            }
            None => contact_info_splits = vec![],
        }

        if !contact_info_splits.is_empty() {
            contact_info_vec = get_contact_info(self.id, contact_info_splits)?;
        }
        self.contact_info = contact_info_vec;

        self.activities = activities;
        self.reminders = reminders;

        Ok(self)
    }

    pub fn parse_from_editor(content: &str) -> Result<EditorData, CliError> {
        let mut error: Option<CliError> = None;
        let mut name: String = String::new();
        let mut birthday: Option<String> = None;
        let mut contact_info: Vec<String> = vec![];
        let mut activity_ids: Vec<u64> = vec![];
        let mut reminder_ids: Vec<u64> = vec![];
        let name_prefix = "Name: ";
        let birthday_prefix = "Birthday: ";
        let contact_info_prefix = "Contact Info: ";
        let activities_prefix = "Activities: ";
        let reminders_prefix = "Reminders: ";
        content.lines().for_each(|line: &str| match line {
            s if s.starts_with(name_prefix) => {
                name = s.trim_start_matches(name_prefix).to_string();
            }
            s if s.starts_with(birthday_prefix) => {
                birthday = Some(s.trim_start_matches(birthday_prefix).to_string());
            }
            s if s.starts_with(contact_info_prefix) => {
                let contact_info_str = s.trim_start_matches(contact_info_prefix);
                contact_info = contact_info_str.split(',').map(|x| x.to_string()).collect();
            }
            s if s.starts_with(activities_prefix) => {
                let activities_str = s.trim_start_matches(activities_prefix);
                let ids = activities_str.split(',').map(|x| x.parse()).collect();
                match ids {
                    Ok(ids) => activity_ids = ids,
                    Err(_) => error = Some(CliError::InvalidIdFormat),
                }
            }
            s if s.starts_with(reminders_prefix) => {
                let reminders_str = s.trim_start_matches(reminders_prefix);
                let ids = reminders_str.split(',').map(|x| x.parse()).collect();
                match ids {
                    Ok(ids) => reminder_ids = ids,
                    Err(_) => error = Some(CliError::InvalidIdFormat),
                }
            }
            _ => error = Some(CliError::FormatError),
        });

        if let Some(error) = error {
            return Err(error);
        }

        Ok(EditorData {
            name,
            birthday,
            contact_info,
            activities: activity_ids,
            reminders: reminder_ids,
        })
    }

    fn update_contact_info(conn: &Connection, person: &Person) -> Result<(), DbOperationsError> {
        if !person.contact_info.is_empty() {
            for ci in person.contact_info.iter() {
                let (ci_type, ci_value): (String, &str) = match &ci.contact_info_type {
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
                let mut stmt = match conn
                    .prepare("SELECT id FROM contact_info_types WHERE type = ?")
                {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };

                let mut rows = match stmt.query(params![ci_type]) {
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

                // Check if entry needs to be updated
                let mut stmt = match conn.prepare(
                    "SELECT EXISTS(SELECT
                        *
                    FROM
                        contact_info
                    WHERE
                        person_id = ?1 AND
                        contact_info_type_id = ?2 AND
                        contact_info_details = ?3 AND
                        deleted = 0)",
                ) {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };
                let mut rows = match stmt.query(params![person.id, types[0], ci_value]) {
                    Ok(rows) => rows,
                    Err(_) => return Err(DbOperationsError::QueryError),
                };

                match rows.next() {
                    Ok(row) => match row {
                        Some(row) => match row.get::<usize, bool>(0) {
                            Ok(exists) => {
                                if !exists {
                                    let mut stmt = match conn.prepare(
                                        "INSERT
                                                    INTO
                                            contact_info
                                               (
                                                   person_id,
                                                   contact_info_type_id,
                                                   contact_info_details,
                                                   deleted
                                               )
                                            VALUES
                                               (?1, ?2, ?3, 0)",
                                    ) {
                                        Ok(stmt) => stmt,
                                        Err(e) => {
                                            return Err(DbOperationsError::InvalidStatement {
                                                sqlite_error: e,
                                            });
                                        }
                                    };
                                    match stmt.execute(params![person.id, types[0], ci_value]) {
                                        Ok(updated) => {
                                            println!(
                                                "[DEBUG][contact_info][insert] {} rows were updated",
                                                updated
                                            )
                                        }
                                        Err(_) => return Err(DbOperationsError::QueryError),
                                    }
                                }
                            }
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        },
                        None => return Err(DbOperationsError::QueryError),
                    },
                    Err(_) => return Err(DbOperationsError::QueryError),
                }
            }

            let mut stmt = match conn.prepare(
                "SELECT
                    *
                FROM
                    contact_info
                WHERE
                    person_id = ?1 AND
                    deleted = 0",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let mut rows = match stmt.query(params![person.id]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            let mut contact_infos: Vec<ContactInfo> = Vec::new();
            loop {
                match rows.next() {
                    Ok(row) => match row {
                        Some(row) => {
                            let id = match row.get(0) {
                                Ok(id) => id,
                                Err(e) => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: Some(e),
                                        strum_error: None,
                                    });
                                }
                            };
                            let ci_id = match row.get(2) {
                                Ok(contact_info_type) => contact_info_type,
                                Err(e) => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: Some(e),
                                        strum_error: None,
                                    })
                                }
                            };
                            let contact_info_type = match ContactInfoType::get_by_id(conn, ci_id)? {
                                Some(contact_info_type) => contact_info_type,
                                None => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: None,
                                        strum_error: None,
                                    })
                                }
                            };

                            let details = match row.get::<usize, String>(3) {
                                Ok(details) => details,
                                Err(e) => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: Some(e),
                                        strum_error: None,
                                    })
                                }
                            };
                            contact_infos.push(ContactInfo {
                                id,
                                person_id: person.id,
                                contact_info_type,
                                details,
                            })
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

            let contact_info_tuples: Vec<(String, String)> = person
                .contact_info
                .iter()
                .map(|ci| {
                    (
                        ci.contact_info_type.clone().as_ref().to_string(),
                        ci.details.clone(),
                    )
                })
                .collect::<Vec<(String, String)>>();
            for contact_info in contact_infos {
                let pair = (
                    contact_info.contact_info_type.as_ref().to_string(),
                    contact_info.details,
                );
                if contact_info_tuples.contains(&pair) {
                    continue;
                }
                let mut stmt = match conn.prepare(
                    "UPDATE
                            contact_info
                        SET
                            deleted = 1
                        WHERE
                            id = ?1",
                ) {
                    Ok(stmt) => stmt,
                    Err(e) => {
                        return Err(DbOperationsError::InvalidStatement { sqlite_error: e });
                    }
                };
                match stmt.execute(params![contact_info.id]) {
                    Ok(updated) => {
                        println!(
                            "[DEBUG][contact_info][update] {} rows were updated",
                            updated
                        )
                    }
                    Err(_) => return Err(DbOperationsError::QueryError),
                }
            }
        }

        Ok(())
    }

    fn update_activities(conn: &Connection, person: &Person) -> Result<(), DbOperationsError> {
        // Check if any activity needs to be added
        for activity in person.activities.clone().iter_mut() {
            let mut stmt = match conn.prepare(
                "SELECT EXISTS(SELECT
                    *
                FROM
                    people_activities
                WHERE
                    activity_id = ?1 AND
                    person_id = ?2 AND
                    deleted = 0)",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            let mut rows = match stmt.query(params![activity.id, person.id]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            match rows.next() {
                Ok(row) => match row {
                    Some(row) => {
                        match row.get::<usize, bool>(0) {
                            Ok(exists) => {
                                if !exists {
                                    activity.people.push(person.clone());
                                    activity.save(conn)?;
                                }
                            }
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                });
                            }
                        };
                    }
                    None => return Err(DbOperationsError::QueryError),
                },
                Err(_) => return Err(DbOperationsError::QueryError),
            }
        }

        // Remove activities
        let mut stmt = match conn.prepare(
            "SELECT
                activity_id
            FROM
                people_activities
            WHERE
                person_id = ? AND
                deleted = 0",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![person.id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut ids: Vec<u64> = Vec::new();
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => {
                        let id: u32 = match row.get(0) {
                            Ok(row) => row,
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        };
                        ids.push(id.into());
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

        let person_activity_ids: Vec<u64> =
            person.activities.iter().map(|a| a.id).collect::<Vec<u64>>();
        for id in ids.iter() {
            if !person_activity_ids.contains(id) {
                let mut stmt = match conn.prepare(
                    "UPDATE
                        people_activities
                    SET
                        deleted = 1
                    WHERE
                        activity_id = ?1 AND
                        person_id = ?2",
                ) {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };
                match stmt.execute(params![id, person.id]) {
                    Ok(updated) => {
                        println!(
                            "[DEBUG][people_activities][update] {} rows were updated",
                            updated
                        );
                    }
                    Err(_) => return Err(DbOperationsError::GenericError),
                }
            }
        }

        Ok(())
    }

    pub fn update_reminders(conn: &Connection, person: &Person) -> Result<(), DbOperationsError> {
        for reminder in person.reminders.clone().iter_mut() {
            let mut stmt = match conn.prepare(
                "SELECT EXISTS(SELECT
                    *
                FROM
                    people_reminders
                WHERE
                    reminder_id = ?1 AND
                    person_id = ?2 AND
                    deleted = 0)",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            let mut rows = match stmt.query(params![reminder.id, person.id]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            match rows.next() {
                Ok(row) => match row {
                    Some(row) => {
                        match row.get::<usize, bool>(0) {
                            Ok(exists) => {
                                if !exists {
                                    reminder.people.push(person.clone());
                                    reminder.save(conn)?;
                                }
                            }
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                });
                            }
                        };
                    }
                    None => return Err(DbOperationsError::QueryError),
                },
                Err(_) => return Err(DbOperationsError::QueryError),
            }
        }

        // Remove reminders
        let mut stmt = match conn.prepare(
            "SELECT
                reminder_id
            FROM
                people_reminders
            WHERE
                person_id = ? AND
                deleted = 0",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut rows = match stmt.query(params![person.id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut ids: Vec<u64> = Vec::new();
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => {
                        let id: u32 = match row.get(0) {
                            Ok(row) => row,
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        };
                        ids.push(id.into());
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

        let person_reminder_ids: Vec<u64> =
            person.reminders.iter().map(|r| r.id).collect::<Vec<u64>>();
        for id in ids.iter() {
            if !person_reminder_ids.contains(id) {
                let mut stmt = match conn.prepare(
                    "UPDATE
                        people_reminders
                    SET
                        deleted = 1
                    WHERE
                        reminder_id = ?1 AND
                        person_id = ?2",
                ) {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };
                match stmt.execute(params![id, person.id]) {
                    Ok(updated) => {
                        println!(
                            "[DEBUG][people_reminders][update] {} rows were updated",
                            updated
                        );
                    }
                    Err(_) => return Err(DbOperationsError::GenericError),
                }
            }
        }
        Ok(())
    }
}

impl crate::db::db_interface::DbOperations for Person {
    fn add(&self, conn: &Connection) -> Result<&Person, DbOperationsError> {
        let mut stmt = match conn
            .prepare("SELECT id FROM people WHERE name = ? AND deleted = 0 COLLATE NOCASE")
        {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::QueryError),
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

        // TODO make all db operations atomic
        let birthday_str = match self.birthday {
            Some(birthday) => birthday.to_string(),
            None => "".to_string(),
        };

        let mut stmt = match conn
            .prepare("INSERT INTO people (name, birthday, deleted) VALUES (?1, ?2, FALSE)")
        {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };

        match stmt.execute(params![self.name, birthday_str]) {
            Ok(updated) => {
                println!("[DEBUG][people][insert] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }
        let id = conn.last_insert_rowid();

        let res = self.contact_info.iter().try_for_each(|contact_info| {
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

            let mut stmt = match conn.prepare("SELECT id FROM contact_info_types WHERE type = ?") {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            let mut rows = match stmt.query(params![ci_type]) {
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
                "INSERT INTO contact_info (
                    person_id,
                    contact_info_type_id,
                    contact_info_details,
                    deleted
                )
                    VALUES (?1, ?2, ?3, FALSE)",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            match stmt.execute(params![id, types[0], ci_value]) {
                Ok(updated) => println!(
                    "[DEBUG][contact_info][insert] {} rows were updated",
                    updated
                ),
                Err(_) => return Err(DbOperationsError::QueryError),
            }
            Ok(())
        });

        res?;

        Ok(self)
    }

    fn remove(&self, conn: &Connection) -> Result<&Person, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "UPDATE
                    people
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
                println!("[DEBUG][people][update] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        Ok(self)
    }

    fn save(&self, conn: &Connection) -> Result<&Person, DbOperationsError> {
        let birthday_str = match self.birthday {
            Some(birthday) => birthday.to_string(),
            None => "".to_string(),
        };

        let mut stmt = match conn.prepare(
            "UPDATE
                people
            SET
                name = ?1,
                birthday = ?2
            WHERE
                id = ?3",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        match stmt.execute(params![self.name, birthday_str, self.id]) {
            Ok(updated) => {
                println!("[DEBUG][people][update] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        Person::update_contact_info(conn, self)?;
        Person::update_activities(conn, self)?;
        Person::update_reminders(conn, self)?;

        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Result<Option<Entities>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM people WHERE id = ?1 AND deleted = 0") {
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
                    let person_id = match row.get(0) {
                        Ok(person_id) => person_id,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let name = match row.get(1) {
                        Ok(name) => name,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let notes = crate::db::db_helpers::notes::get_by_person(conn, person_id)?;
                    let reminders =
                        crate::db::db_helpers::reminders::get_by_person_reminders(conn, person_id)?;
                    let contact_info =
                        crate::db::db_helpers::contact_info::get_by_person(conn, person_id)?;
                    let activities =
                        crate::db::db_helpers::activities::get_by_person(conn, person_id)?;
                    Ok(Some(Entities::Person(Person {
                        id: person_id,
                        name,
                        birthday: Some(
                            crate::helpers::parse_from_str_ymd(
                                row.get::<usize, String>(2).unwrap_or_default().as_str(),
                            )
                            .unwrap_or_default(),
                        ),
                        contact_info,
                        activities,
                        reminders,
                        notes,
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
        let mut stmt = match conn.prepare("SELECT * FROM people WHERE deleted = 0 COLLATE NOCASE") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };

        let rows = match stmt.query_map([], |row| {
            let person_id = row.get(0)?;
            let notes = match crate::db::db_helpers::notes::get_by_person(conn, person_id) {
                Ok(notes) => notes,
                Err(e) => {
                    let sqlite_error = match e {
                        DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                        other => panic!("Unexpected error type: {:#?}", other),
                    };
                    return Err(sqlite_error);
                }
            };
            let reminders =
                match crate::db::db_helpers::reminders::get_by_person_reminders(conn, person_id) {
                    Ok(reminders) => reminders,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
            let contact_info =
                match crate::db::db_helpers::contact_info::get_by_person(conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
            let activities = match crate::db::db_helpers::activities::get_by_person(conn, person_id)
            {
                Ok(activities) => activities,
                Err(e) => {
                    let sqlite_error = match e {
                        DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                        other => panic!("Unexpected error type: {:#?}", other),
                    };
                    return Err(sqlite_error);
                }
            };
            Ok(Person {
                id: person_id,
                name: row.get(1)?,
                birthday: Some(
                    crate::helpers::parse_from_str_ymd(
                        row.get::<usize, String>(2).unwrap_or_default().as_str(),
                    )
                    .unwrap_or_default(),
                ),
                contact_info,
                activities,
                reminders,
                notes,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut people = Vec::new();

        for person in rows.into_iter() {
            let person = match person {
                Ok(person) => person,
                Err(e) => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: Some(e),
                        strum_error: None,
                    })
                }
            };
            people.push(Box::new(person));
        }

        Ok(people)
    }
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let birthday: String = match &self.birthday {
            Some(bday) => bday.to_string(),
            None => String::new(),
        };
        let mut contact_info_str = String::new();
        for ci in self.contact_info.iter() {
            contact_info_str.push_str("\n\t");
            contact_info_str.push_str(ci.contact_info_type.as_ref());
            contact_info_str.push_str(": ");
            contact_info_str.push_str(ci.details.as_ref());
        }
        let mut activities_str = String::new();
        for activity in self.activities.iter() {
            activities_str.push_str("\n\t");
            activities_str.push_str(format!("name: {}\n\t", activity.name).as_ref());
            activities_str.push_str(format!("date: {}\n\t", activity.date).as_ref());
            activities_str.push_str(
                format!("activity type: {}\n\t", activity.activity_type.as_ref()).as_ref(),
            );
            activities_str.push_str(format!("content: {}\n\t", activity.content).as_ref());
            let people = activity
                .people
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<&str>>()
                .join(",");
            activities_str.push_str(format!("people: {}\n\t", people).as_ref());
        }
        let mut reminders_str = String::new();
        for reminder in self.reminders.iter() {
            reminders_str.push_str("\n\t");
            reminders_str.push_str(format!("name: {}\n\t", reminder.name).as_ref());
            reminders_str.push_str(format!("date: {}\n\t", reminder.date).as_ref());
            if let Some(description) = reminder.clone().description {
                reminders_str.push_str(format!("description: {}\n\t", description).as_ref());
            }
        }
        let mut notes_str = String::new();
        for note in self.notes.iter() {
            notes_str.push_str("\n\t");
            notes_str.push_str(format!("date: {}\n\t", note.date).as_ref());
            notes_str.push_str(format!("content: {}\n\t", note.content).as_ref());
        }
        write!(
            f,
            "person id: {}\nname: {}\nbirthday: {}\ncontact_info: {}\nactivities: {}\nreminders: {}\nnotes: {}\n",
            &self.id, &self.name, birthday, contact_info_str, activities_str, reminders_str, notes_str,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContactInfo {
    id: u64,
    person_id: u64,
    pub contact_info_type: ContactInfoType,
    pub details: String,
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

    pub fn populate_splits(splits: &mut Vec<Vec<String>>, list: &mut [String]) {
        list.iter_mut().for_each(|contact_info_str| {
            splits.push(contact_info_str.split(':').map(|x| x.to_string()).collect());
        });
    }
}

#[derive(Debug, AsRefStr, EnumString, Clone, PartialEq)]
pub enum ContactInfoType {
    Phone(String),
    WhatsApp(String),
    Email(String),
}

impl ContactInfoType {
    pub fn get_by_id(
        conn: &Connection,
        id: u64,
    ) -> Result<Option<ContactInfoType>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT type FROM contact_info_types WHERE id = ?") {
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
                    let ci_type_id = match row.get::<usize, String>(0) {
                        Ok(ci_type_id) => ci_type_id,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let ci = match ContactInfoType::from_str(ci_type_id.as_str()) {
                        Ok(ci) => ci,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: None,
                                strum_error: Some(e),
                            })
                        }
                    };
                    Ok(Some(ci))
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
        let name = String::from("Zeh");
        let birthday = crate::helpers::parse_from_str_ymd("2000-01-01").unwrap();
        let contact_info: Vec<ContactInfo> = vec![];
        let activities: Vec<Activity> = vec![];
        let reminders: Vec<Reminder> = vec![];
        let notes: Vec<Note> = vec![];

        let person = Person::new(
            id,
            name.clone(),
            Some(birthday),
            contact_info.clone(),
            activities.clone(),
            reminders.clone(),
        );

        assert_eq!(
            Person {
                id,
                name,
                birthday: Some(birthday),
                contact_info,
                activities,
                reminders,
                notes,
            },
            person
        );
    }
}
