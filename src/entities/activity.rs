use chrono::prelude::*;
use rusqlite::{params, Connection};
use std::{convert::AsRef, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::entities::person::Person;
use crate::entities::Entities;

pub static ACTIVITY_TEMPLATE: &str = "Name: {name}
Date: {date}
Activity Type: {activity_type}
Content: {content}
People: {people}
";
#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    id: u64,
    pub name: String,
    pub activity_type: ActivityType,
    pub date: NaiveDate,
    pub content: String,
    pub people: Vec<Person>,
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
                        activity_type: ActivityType::get_by_id(&conn, row.get(2).unwrap()).unwrap(),
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        content: row.get(4).unwrap(),
                        people: crate::db::db_helpers::get_people_by_activity(
                            &conn,
                            activity_id,
                            true,
                        ),
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
                    activity_type: ActivityType::get_by_id(&conn, row.get(2).unwrap()).unwrap(),
                    date: crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    content: row.get(4).unwrap(),
                    people: crate::db::db_helpers::get_people_by_activity(&conn, activity_id, true),
                })
            })
            .unwrap();

        let mut activities = Vec::new();

        for activity in rows.into_iter() {
            activities.push(activity.unwrap());
        }

        activities
    }

    pub fn update(
        &mut self,
        conn: &Connection,
        name: Option<String>,
        activity_type: Option<String>,
        date: Option<String>,
        content: Option<String>,
        people: Vec<String>,
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
        let mut activity_type: Option<String> = None;
        let mut activity_content: Option<String> = None;
        let mut people: Vec<String> = Vec::new();

        let name_prefix = "Name: ";
        let date_prefix = "Date: ";
        let activity_type_prefix = "Activity Type: ";
        let activity_content_prefix = "Content: ";
        let people_prefix = "People: ";

        content.lines().for_each(|line| match line {
            s if s.starts_with(name_prefix) => {
                name = s.trim_start_matches(name_prefix).to_string();
            }
            s if s.starts_with(date_prefix) => {
                date = Some(s.trim_start_matches(date_prefix).to_string());
            }
            s if s.starts_with(activity_type_prefix) => {
                activity_type = Some(s.trim_start_matches(activity_type_prefix).to_string());
            }
            s if s.starts_with(activity_content_prefix) => {
                activity_content = Some(s.trim_start_matches(activity_content_prefix).to_string());
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

        Ok((name, date, activity_type, activity_content, people))
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

    fn remove(
        &self,
        conn: &Connection,
    ) -> Result<&Self, crate::db::db_interface::DbOperationsError> {
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

        for person in self.people.iter() {
            let mut stmt = conn
                .prepare(
                    "SELECT 
                        id
                    FROM
                        people_activities
                    WHERE
                        activity_id = ?1 
                        AND person_id = ?2",
                )
                .unwrap();
            let mut rows = stmt.query(params![self.id, person.id]).unwrap();
            let mut results: Vec<u32> = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                results.push(row.get(0).unwrap());
            }

            if results.len() > 0 {
                for id in results {
                    match conn.execute(
                        "UPDATE people_activities SET deleted = TRUE WHERE id = ?1",
                        params![id],
                    ) {
                        Ok(updated) => {
                            println!("[DEBUG] {} rows were updated", updated);
                        }
                        Err(_) => {
                            return Err(crate::db::db_interface::DbOperationsError::GenericError)
                        }
                    }
                }
            }

            match conn.execute(
                "INSERT INTO people_activities (
                        person_id,
                        activity_id,
                        deleted
                    ) VALUES (?1, ?2, FALSE)",
                params![person.id, self.id],
            ) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
            }
        }

        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Option<Entities> {
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
                        activity_type: ActivityType::get_by_id(&conn, row.get(2).unwrap()).unwrap(),
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        content: row.get(4).unwrap(),
                        people: crate::db::db_helpers::get_people_by_activity(
                            &conn,
                            activity_id,
                            true,
                        ),
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }
}

#[derive(Debug, AsRefStr, EnumString, Clone, PartialEq)]
pub enum ActivityType {
    Phone,
    InPerson,
    Online,
}

impl ActivityType {
    pub fn get_by_id(conn: &Connection, id: u64) -> Option<ActivityType> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let id = 1;
        let name = "hiking".to_string();
        let activity_type = ActivityType::InPerson;
        let date = crate::helpers::parse_from_str_ymd("2018-01-01").unwrap();
        let content = "hiking the mountains".to_string();
        let people: Vec<Person> = vec![];
        let activity = Activity::new(
            id,
            name.clone(),
            activity_type.clone(),
            date,
            content.clone(),
            people.clone(),
        );
        assert_eq!(
            Activity {
                id,
                name,
                activity_type,
                date,
                content,
                people
            },
            activity
        );
    }

    #[test]
    fn test_get_by_names() {
        let name = "cycling";
        let conn = Connection::open("data/prm_test.db").unwrap();

        let result = Activity::get_by_name(&conn, name);
        assert!(result.is_none());
    }
}
