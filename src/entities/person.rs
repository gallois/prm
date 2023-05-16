use chrono::prelude::*;
use rusqlite::{params, params_from_iter, Connection};
use std::{convert::AsRef, fmt, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::entities::activity::Activity;
use crate::entities::note::Note;
use crate::entities::reminder::Reminder;
use crate::entities::Entities;

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
                    // TODO proper error handling and messaging
                    _ => panic!("Unknown contact info type"),
                }
                if let Some(contact_info_type) = contact_info_type {
                    contact_info_vec.push(ContactInfo::new(0, self.id, contact_info_type));
                }
            }
        }
        self.contact_info = contact_info_vec;

        self
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
            // FIXME
            _ => error = true,
        });

        if error {
            return Err(crate::editor::ParseError::FormatError);
        }

        Ok((name, birthday, contact_info))
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
                    .prepare("SELECT id FROM contact_info WHERE person_id = ?1 AND contact_info_type_id = ?2")
                    .unwrap();
                let mut rows = stmt.query(params![self.id, types[0]]).unwrap();
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
        }

        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Option<Entities> {
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
    pub fn get_by_id(conn: &Connection, id: u64) -> Option<ContactInfoType> {
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
