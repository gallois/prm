use chrono::prelude::*;
use rusqlite::{params, params_from_iter};
use std::{convert::AsRef, fmt, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::db::db_interface::DbOperationsError;
use crate::entities::activity::Activity;
use crate::entities::note::Note;
use crate::entities::reminder::Reminder;
use crate::entities::Entities;
use rusqlite::Connection;

use snafu::prelude::*;

// FIXME this is a duplication of what we have in `CliError` (src/cli/add.rs)
#[derive(Debug, Snafu)]
pub enum EntityError {
    #[snafu(display("Invalid birthday: {}", birthday))]
    BirthdayParseError { birthday: String },
    #[snafu(display("Invalid contact info: {}", contact_info))]
    ContactInfoParseError { contact_info: String },
}

pub static PERSON_TEMPLATE: &str = "Name: {name}
Birthday: {birthday}
Contact Info: {contact_info}
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

    // TODO create a separate function for additional filters
    pub fn get_by_name(
        conn: &Connection,
        name: Option<String>,
        birthday: Option<String>,
    ) -> Result<Option<Person>, DbOperationsError> {
        let mut query = String::from("SELECT * FROM people WHERE");
        let mut name_present: bool = false;
        let mut birthday_present: bool = false;
        let mut name_some: String = String::from("");
        let mut birthday_some: String = String::from("");
        if let Some(name) = name {
            name_present = true;
            query = query + " name = ?1";
            name_some = name;
        }
        if let Some(birthday) = birthday {
            let mut placeholder = "?1";
            if name_present {
                query = query + " AND";
                placeholder = "?2";
            }
            birthday_present = true;
            query = query + " birthday LIKE " + placeholder;
            birthday_some = "%".to_string() + &birthday;
        }
        query = query + " COLLATE NOCASE";

        let mut stmt = match conn.prepare(query.as_str()) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let mut query_params = vec![];
        if name_present {
            query_params.push(name_some.as_str());
        }
        if birthday_present {
            query_params.push(birthday_some.as_str());
        }
        let mut rows = match stmt.query(params_from_iter(query_params)) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        // TODO allow for returning multiple rows
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
                    let notes = crate::db::db_helpers::get_notes_by_person(&conn, person_id)?;
                    let reminders =
                        crate::db::db_helpers::get_reminders_by_person(&conn, person_id)?;
                    let contact_info =
                        crate::db::db_helpers::get_contact_info_by_person(&conn, person_id)?;
                    let activities =
                        crate::db::db_helpers::get_activities_by_person(&conn, person_id)?;

                    Ok(Some(Person {
                        id: person_id,
                        name: name,
                        birthday: Some(
                            crate::helpers::parse_from_str_ymd(
                                String::from(row.get::<usize, String>(2).unwrap_or_default())
                                    .as_str(),
                            )
                            .unwrap_or_default(),
                        ),
                        contact_info: contact_info,
                        activities: activities,
                        reminders: reminders,
                        notes: notes,
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

    pub fn get_by_names(
        conn: &Connection,
        names: Vec<String>,
    ) -> Result<Vec<Person>, DbOperationsError> {
        if names.is_empty() {
            return Ok(vec![]);
        }

        let vars = crate::helpers::repeat_vars(names.len());
        let sql = format!(
            "SELECT * FROM people WHERE name IN ({}) COLLATE NOCASE",
            vars
        );

        let mut people = vec![];
        let mut stmt = match conn.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };
        let rows: _ = match stmt.query_map(params_from_iter(names.iter()), |row| {
            Ok(Person::new(
                row.get(0)?,
                row.get(1)?,
                Some(
                    crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                ),
                vec![],
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

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
            people.push(person);
        }

        Ok(people)
    }

    pub fn get_all(conn: &Connection) -> Result<Vec<Person>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM people") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };

        let rows = match stmt.query_map([], |row| {
            let person_id = row.get(0)?;
            let notes = match crate::db::db_helpers::get_notes_by_person(&conn, person_id) {
                Ok(notes) => notes,
                Err(e) => {
                    let sqlite_error = match e {
                        DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                        other => panic!("Unexpected error type: {:#?}", other),
                    };
                    return Err(sqlite_error);
                }
            };
            let reminders = match crate::db::db_helpers::get_reminders_by_person(&conn, person_id) {
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
                match crate::db::db_helpers::get_contact_info_by_person(&conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
            let activities = match crate::db::db_helpers::get_activities_by_person(&conn, person_id)
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
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                ),
                contact_info: contact_info,
                activities: activities,
                reminders: reminders,
                notes: notes,
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
            people.push(person);
        }

        Ok(people)
    }

    // TODO might be a good idea to edit activities, reminders and notes vectors
    pub fn update(
        &mut self,
        name: Option<String>,
        birthday: Option<String>,
        contact_info: Option<String>,
    ) -> Result<&Self, EntityError> {
        // TODO clean up duplication between this and main.rs
        if let Some(name) = name {
            self.name = name;
        }
        if let Some(birthday) = birthday {
            let birthday_obj: Option<NaiveDate>;
            match crate::helpers::parse_from_str_ymd(&birthday) {
                Ok(date) => birthday_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&birthday) {
                    Ok(date) => birthday_obj = Some(date),
                    Err(_) => return BirthdayParseSnafu { birthday: birthday }.fail(),
                },
            }
            self.birthday = birthday_obj;
        }

        let mut contact_info_splits: Vec<Vec<String>> = vec![];
        let mut contact_info_type: Option<ContactInfoType>;
        let mut contact_info_vec: Vec<ContactInfo> = Vec::new();
        match contact_info {
            Some(contact_info_vec) => {
                for contact_info_str in contact_info_vec.split(",") {
                    contact_info_splits
                        .push(contact_info_str.split(":").map(|x| x.to_string()).collect());
                }
            }
            None => contact_info_splits = vec![],
        }

        // FIXME duplication in src/cli/add.rs
        let mut invalid_contact_info = vec![];
        if contact_info_splits.len() > 0 {
            for contact_info_split in contact_info_splits.iter() {
                match contact_info_split[0].as_str() {
                    "phone" => {
                        contact_info_type =
                            Some(ContactInfoType::Phone(contact_info_split[1].clone()))
                    }
                    "whatsapp" => {
                        contact_info_type =
                            Some(ContactInfoType::WhatsApp(contact_info_split[1].clone()))
                    }
                    "email" => {
                        contact_info_type =
                            Some(ContactInfoType::Email(contact_info_split[1].clone()))
                    }
                    _ => {
                        invalid_contact_info.push(
                            vec![contact_info_split[0].clone(), contact_info_split[1].clone()]
                                .join(":"),
                        );
                        return ContactInfoParseSnafu {
                            contact_info: String::from(invalid_contact_info.join(",")),
                        }
                        .fail();
                    }
                }

                if let Some(contact_info_type) = contact_info_type {
                    contact_info_vec.push(ContactInfo::new(0, self.id, contact_info_type));
                }
            }
        }
        self.contact_info = contact_info_vec;

        Ok(self)
    }

    pub fn parse_from_editor(
        content: &str,
    ) -> Result<(String, Option<String>, Vec<String>), crate::editor::ParseError> {
        let mut error = false;
        let mut name: String = String::new();
        let mut birthday: Option<String> = None;
        let mut contact_info: Vec<String> = vec![];
        let name_prefix = "Name: ";
        let birthday_prefix = "Birthday: ";
        let contact_info_prefix = "Contact Info: ";
        content.lines().for_each(|line| match line {
            s if s.starts_with(name_prefix) => {
                name = s.trim_start_matches(name_prefix).to_string();
            }
            s if s.starts_with(birthday_prefix) => {
                birthday = Some(s.trim_start_matches(birthday_prefix).to_string());
            }
            s if s.starts_with(contact_info_prefix) => {
                let contact_info_str = s.trim_start_matches(contact_info_prefix);
                contact_info = contact_info_str.split(",").map(|x| x.to_string()).collect();
            }
            _ => error = true,
        });

        if error {
            return Err(crate::editor::ParseError::FormatError);
        }

        Ok((name, birthday, contact_info))
    }
}

impl crate::db::db_interface::DbOperations for Person {
    fn add(&self, conn: &Connection) -> Result<&Person, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT id FROM people WHERE name = ?") {
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

        if ids.len() > 0 {
            return Err(crate::db::db_interface::DbOperationsError::DuplicateEntry);
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
                println!("[DEBUG] {} rows were updated", updated);
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
                Err(_) => return Err(crate::db::db_interface::DbOperationsError::QueryError),
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
                Ok(updated) => println!("[DEBUG] {} rows were updated", updated),
                Err(_) => return Err(DbOperationsError::QueryError),
            }
            Ok(())
        });

        if res.is_err() {
            return Err(res.unwrap_err());
        }

        Ok(self)
    }

    fn remove(
        &self,
        conn: &Connection,
    ) -> Result<&Person, crate::db::db_interface::DbOperationsError> {
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
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
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
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        if self.contact_info.len() > 0 {
            for ci in self.contact_info.iter() {
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

                let mut stmt = match conn.prepare("SELECT id FROM contact_info WHERE person_id = ?1 AND contact_info_type_id = ?2") {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };
                let mut rows = match stmt.query(params![self.id, types[0]]) {
                    Ok(rows) => rows,
                    Err(_) => return Err(DbOperationsError::QueryError),
                };
                let mut ci_ids: Vec<u32> = Vec::new();
                loop {
                    match rows.next() {
                        Ok(row) => match row {
                            Some(row) => match row.get(0) {
                                Ok(row) => ci_ids.push(row),
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
                    contact_info 
                SET
                    person_id = ?1,
                    contact_info_type_id = ?2,
                    contact_info_details = ?3
                WHERE
                    id = ?4",
                ) {
                    Ok(stmt) => stmt,
                    Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
                };
                match stmt.execute(params![self.id, types[0], ci_value, ci_ids[0]]) {
                    Ok(updated) => {
                        println!("[DEBUG] {} rows were updated", updated);
                    }
                    Err(_) => return Err(DbOperationsError::QueryError),
                }
            }
        }

        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Result<Option<Entities>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM people WHERE id = ?1") {
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
                    let notes = crate::db::db_helpers::get_notes_by_person(&conn, person_id)?;
                    let reminders =
                        crate::db::db_helpers::get_reminders_by_person(&conn, person_id)?;
                    let contact_info =
                        crate::db::db_helpers::get_contact_info_by_person(&conn, person_id)?;
                    let activities =
                        crate::db::db_helpers::get_activities_by_person(&conn, person_id)?;
                    Ok(Some(Entities::Person(Person {
                        id: person_id,
                        name,
                        birthday: Some(
                            crate::helpers::parse_from_str_ymd(
                                String::from(row.get::<usize, String>(2).unwrap_or_default())
                                    .as_str(),
                            )
                            .unwrap_or_default(),
                        ),
                        contact_info: contact_info,
                        activities: activities,
                        reminders: reminders,
                        notes: notes,
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

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let birthday: String;
        match &self.birthday {
            Some(bday) => birthday = bday.to_string(),
            None => birthday = String::new(),
        }
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
        // TODO implement remaining fields
        write!(
            f,
            "person id: {}\nname: {}\nbirthday: {}\ncontact_info: {}\nactivities: {}\n",
            &self.id, &self.name, birthday, contact_info_str, activities_str
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

    pub fn populate_splits(splits: &mut Vec<Vec<String>>, list: &mut Vec<String>) {
        list.into_iter().for_each(|contact_info_str| {
            splits.push(contact_info_str.split(":").map(|x| x.to_string()).collect());
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
                    let ci = match ContactInfoType::from_str(&ci_type_id.as_str()) {
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
        let name = String::from("Zeh");
        let birthday = crate::helpers::parse_from_str_ymd("2000-01-01").unwrap();
        let contact_info: Vec<ContactInfo> = vec![];
        let activities: Vec<Activity> = vec![];
        let reminders: Vec<Reminder> = vec![];
        let notes: Vec<Note> = vec![];

        let person = Person::new(id, name.clone(), Some(birthday), contact_info.clone());

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
