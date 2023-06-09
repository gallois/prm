use chrono::prelude::*;
use rusqlite::params;
use std::{convert::AsRef, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

use crate::db_interface::{DbOperations, DbOperationsError};
use crate::entities::person::Person;
use crate::entities::Entities;
use rusqlite::Connection;

use snafu::prelude::*;

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

#[derive(Debug, Snafu)]
pub enum ActivityError {
    #[snafu(display("Invalid activity type: {}", activity_type))]
    ActivityTypeParseError { activity_type: String },
    // FIXME this is a duplication of what we have in `CliError` (src/cli/add.rs)
    #[snafu(display("Invalid date: {}", date))]
    DateParseError { date: String },
    #[snafu(display("Invalid record: {}", record))]
    RecordParseError { record: String },
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

    pub fn get(
        conn: &Connection,
        name: Option<String>,
        person: Option<String>,
    ) -> Result<Vec<Activity>, DbOperationsError> {
        let mut activities: Vec<Activity> = vec![];
        match name {
            Some(name) => {
                let mut stmt = match conn
                    .prepare("SELECT * FROM activities WHERE name = ?1 COLLATE NOCASE")
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
                            let activity_id = match row.get(0) {
                                Ok(activity_id) => activity_id,
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
                            let activity_type_id: u64 = match row.get(2) {
                                Ok(activity_type_id) => activity_type_id,
                                Err(e) => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: Some(e),
                                        strum_error: None,
                                    })
                                }
                            };
                            let content: String = match row.get(4) {
                                Ok(content) => content,
                                Err(e) => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: Some(e),
                                        strum_error: None,
                                    })
                                }
                            };
                            let people = crate::db::db_helpers::get_people_by_activity(
                                &conn,
                                activity_id,
                                true,
                            )?;
                            let activity_type =
                                match ActivityType::get_by_id(&conn, activity_type_id) {
                                    Ok(activity_type) => match activity_type {
                                        Some(activity_type) => activity_type,
                                        None => {
                                            return Err(DbOperationsError::RecordError {
                                                sqlite_error: None,
                                                strum_error: None,
                                            })
                                        }
                                    },
                                    Err(e) => return Err(e),
                                };
                            if let Some(person) = person {
                                let people_name: Vec<String> =
                                    people.iter().map(|p| p.name.to_owned()).collect();
                                if people_name.contains(&person) {
                                    activities.push(Activity {
                                        id: activity_id,
                                        name: name,
                                        activity_type,
                                        date: crate::helpers::parse_from_str_ymd(
                                            String::from(
                                                row.get::<usize, String>(3).unwrap_or_default(),
                                            )
                                            .as_str(),
                                        )
                                        .unwrap_or_default(),
                                        content: content,
                                        people: people,
                                    });
                                }
                            } else {
                                // FIXME remove this duplication
                                activities.push(Activity {
                                    id: activity_id,
                                    name: name,
                                    activity_type,
                                    date: crate::helpers::parse_from_str_ymd(
                                        String::from(
                                            row.get::<usize, String>(3).unwrap_or_default(),
                                        )
                                        .as_str(),
                                    )
                                    .unwrap_or_default(),
                                    content: content,
                                    people: people,
                                });
                            }
                        }
                        None => return Ok(vec![]),
                    },
                    Err(_) => return Err(DbOperationsError::GenericError),
                }
            }
            None => (),
        }
        // TODO handle only passing person
        //      the strategy here is probably to change the original
        //      query to ignore filtering by name and applying the same filters
        //      down the line.
        return Ok(activities);
    }

    pub fn get_all(conn: &Connection) -> Result<Vec<Activity>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM activities") {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::GenericError),
        };

        let rows = match stmt.query_map([], |row| {
            let activity_id = row.get(0)?;
            let people =
                match crate::db::db_helpers::get_people_by_activity(&conn, activity_id, true) {
                    Ok(people) => people,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
            let activity_type = match ActivityType::get_by_id(&conn, row.get(2)?) {
                Ok(activity_type) => match activity_type {
                    Some(activity_type) => activity_type,
                    None => panic!("Activity type cannot be None"),
                },
                Err(e) => {
                    let sqlite_error = match e {
                        DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                        other => panic!("Unexpected error type: {:#?}", other),
                    };
                    return Err(sqlite_error);
                }
            };
            Ok(Activity {
                id: activity_id,
                name: row.get(1)?,
                activity_type,
                date: crate::helpers::parse_from_str_ymd(
                    String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                )
                .unwrap_or_default(),
                content: row.get(4)?,
                people: people,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::GenericError),
        };

        let mut activities = Vec::new();

        for activity in rows.into_iter() {
            let activity = match activity {
                Ok(activity) => activity,
                Err(e) => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: Some(e),
                        strum_error: None,
                    })
                }
            };
            activities.push(activity);
        }

        Ok(activities)
    }

    pub fn update(
        &mut self,
        conn: &Connection,
        name: Option<String>,
        activity_type: Option<String>,
        date: Option<String>,
        content: Option<String>,
        people: Vec<String>,
    ) -> Result<&Self, ActivityError> {
        // TODO clean up duplication between this and main.rs
        if let Some(name) = name {
            self.name = name;
        }

        if let Some(activity_type) = activity_type {
            let activity_type = match activity_type.as_str() {
                "phone" => ActivityType::Phone,
                "in_person" => ActivityType::InPerson,
                "online" => ActivityType::Online,
                _ => {
                    return ActivityTypeParseSnafu {
                        activity_type: activity_type.to_string(),
                    }
                    .fail()
                }
            };

            self.activity_type = activity_type;
        }

        if let Some(date) = date {
            let date_obj: Option<NaiveDate>;
            match crate::helpers::parse_from_str_ymd(&date) {
                Ok(date) => date_obj = Some(date),
                Err(_) => match crate::helpers::parse_from_str_md(&date) {
                    Ok(date) => date_obj = Some(date),
                    Err(_) => {
                        return {
                            DateParseSnafu {
                                date: date.to_string(),
                            }
                            .fail()
                        }
                    }
                },
            }
            self.date = match date_obj {
                Some(date) => date,
                None => {
                    return {
                        DateParseSnafu {
                            date: date.to_string(),
                        }
                        .fail()
                    }
                }
            };
        }

        if let Some(content) = content {
            self.content = content;
        }

        let people = Person::get_by_names(&conn, people);
        self.people = match people {
            Ok(people) => people,
            Err(_) => {
                return {
                    RecordParseSnafu {
                        record: "people".to_string(),
                    }
                    .fail()
                }
            }
        };

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
            _ => error = true,
        });

        if error {
            return Err(crate::editor::ParseError::FormatError);
        }

        Ok((name, date, activity_type, activity_content, people))
    }
}

impl DbOperations for Activity {
    fn add(&self, conn: &Connection) -> Result<&Activity, DbOperationsError> {
        let activity_type_str = self.activity_type.as_ref();
        let date_str = self.date.to_string();

        let mut stmt = match conn.prepare("SELECT id FROM activity_types WHERE type = ?") {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::GenericError),
        };
        let mut rows = match stmt.query(params![activity_type_str]) {
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
                activities (name, type, date, content, deleted)
                VALUES (?1, ?2, ?3, ?4, FALSE)
            ",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::GenericError),
        };

        match stmt.execute(params![self.name, types[0], date_str, self.content]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::GenericError),
        }

        let id = conn.last_insert_rowid();

        for person in &self.people {
            let mut stmt = match conn.prepare(
                "INSERT INTO people_activities (
                    person_id, 
                    activity_id,
                    deleted
                )
                    VALUES (?1, ?2, FALSE)",
            ) {
                Ok(stmt) => stmt,
                Err(_) => return Err(DbOperationsError::GenericError),
            };
            match stmt.execute(params![person.id, id]) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(DbOperationsError::GenericError),
            }
        }

        Ok(self)
    }

    fn remove(&self, conn: &Connection) -> Result<&Self, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "UPDATE 
                    activities 
                SET
                    deleted = TRUE
                WHERE
                    id = ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::GenericError),
        };
        match stmt.execute([self.id]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn save(&self, conn: &Connection) -> Result<&Activity, DbOperationsError> {
        let activity_type_str = self.activity_type.as_ref();

        let mut stmt = match conn.prepare("SELECT id FROM activity_types WHERE type = ?") {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::GenericError),
        };
        let mut rows = match stmt.query(params![activity_type_str]) {
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
                activities
            SET
                name = ?1,
                type = ?2,
                date = ?3,
                content = ?4
            WHERE
                id = ?5",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::GenericError),
        };
        match stmt.execute(params![
            self.name,
            types[0],
            self.date.to_string(),
            self.content,
            self.id,
        ]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::GenericError),
        }

        for person in self.people.iter() {
            let mut stmt = match conn.prepare(
                "SELECT 
                        id
                    FROM
                        people_activities
                    WHERE
                        activity_id = ?1 
                        AND person_id = ?2",
            ) {
                Ok(stmt) => stmt,
                Err(_) => return Err(DbOperationsError::GenericError),
            };
            let mut rows = match stmt.query(params![self.id, person.id]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };
            let mut results: Vec<u32> = Vec::new();
            loop {
                match rows.next() {
                    Ok(row) => match row {
                        Some(row) => match row.get(0) {
                            Ok(row) => results.push(row),
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

            if results.len() > 0 {
                for id in results {
                    let mut stmt = match conn
                        .prepare("UPDATE people_activities SET deleted = TRUE WHERE id = ?1")
                    {
                        Ok(stmt) => stmt,
                        Err(_) => return Err(DbOperationsError::GenericError),
                    };
                    match stmt.execute(params![id]) {
                        Ok(updated) => {
                            println!("[DEBUG] {} rows were updated", updated);
                        }
                        Err(_) => {
                            return Err(crate::db::db_interface::DbOperationsError::GenericError)
                        }
                    }
                }
            }

            let mut stmt = match conn.prepare(
                "INSERT INTO people_activities (
                        person_id,
                        activity_id,
                        deleted
                    ) VALUES (?1, ?2, FALSE)",
            ) {
                Ok(stmt) => stmt,
                Err(_) => return Err(DbOperationsError::GenericError),
            };
            match stmt.execute(params![person.id, self.id]) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(DbOperationsError::GenericError),
            }
        }

        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Result<Option<Entities>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM activities WHERE id = ?1") {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::GenericError),
        };
        let mut rows = match stmt.query(params![id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let activity_id = match row.get(0) {
                        Ok(row) => row,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let name: String = match row.get(1) {
                        Ok(row) => row,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let activity_type_id = match row.get(2) {
                        Ok(row) => row,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let content: String = match row.get(3) {
                        Ok(row) => row,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let people =
                        crate::db::db_helpers::get_people_by_activity(&conn, activity_id, true)?;
                    let activity_type = match ActivityType::get_by_id(&conn, activity_type_id) {
                        Ok(activity_type) => match activity_type {
                            Some(activity_type) => activity_type,
                            None => panic!("Activity type cannot be None"),
                        },
                        Err(e) => return Err(e),
                    };
                    Ok(Some(Entities::Activity(Activity {
                        id: activity_id,
                        name,
                        activity_type,
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        content,
                        people,
                    })))
                }
                None => return Ok(None),
            },
            Err(_) => return Err(DbOperationsError::GenericError),
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
    pub fn get_by_id(
        conn: &Connection,
        id: u64,
    ) -> Result<Option<ActivityType>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT type FROM activity_types WHERE id = ?") {
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
                    let type_str = match row.get::<usize, String>(0) {
                        Ok(type_str) => type_str,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let activity_type = match ActivityType::from_str(type_str.as_str()) {
                        Ok(activity_type) => activity_type,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: None,
                                strum_error: Some(e),
                            })
                        }
                    };
                    Ok(Some(activity_type))
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
}
