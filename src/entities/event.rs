use chrono::prelude::*;
use rusqlite::params;
use std::{convert::AsRef, fmt};

use crate::db::db_interface::DbOperationsError;
use crate::entities::person::Person;
use crate::entities::reminder::{RecurringType, Reminder};
use rusqlite::Connection;

#[derive(Debug)]
pub enum EventError {
    DbError(DbOperationsError),
    EntityError(String),
    DateError,
}

pub enum EventType {
    Person(Person),
    Reminder(Reminder),
}

pub struct Event {
    pub date: NaiveDate,
    kind: String,
    pub details: EventType,
}

impl Event {
    pub fn get_all(conn: &Connection, mut days: u64) -> Result<Vec<Event>, EventError> {
        if days == 0 {
            days = 10 * 365; // 10 years
        }
        let mut events: Vec<Event> = vec![];
        let today = chrono::Local::now().naive_local();
        let today_str = format!("{}", today.format("%Y-%m-%d"));
        let date_limit = match today.checked_add_days(chrono::Days::new(days)) {
            Some(date) => date,
            None => return Err(EventError::DateError),
        };
        let date_limit_str = format!("{}", date_limit.format("%Y-%m-%d"));

        let mut stmt = match conn.prepare(
            "SELECT
                    *,
                    strftime('%j', birthday) - strftime('%j', 'now') AS days_remaining
                FROM
                    people
                WHERE ?1 >= CASE
                    WHEN days_remaining >= 0 THEN days_remaining
                    ELSE days_remaining + strftime('%j', strftime('%Y-12-31', 'now'))
                    END
                AND
                    deleted = 0
                ",
        ) {
            Ok(stmt) => stmt,
            Err(e) => {
                return Err(EventError::DbError(DbOperationsError::InvalidStatement {
                    sqlite_error: e,
                }));
            }
        };

        let rows = match stmt.query_map(params![days], |row| {
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
                contact_info,
                activities,
                reminders,
                notes,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(EventError::DbError(DbOperationsError::QueryError)),
        };
        for person in rows.into_iter() {
            let person = match person {
                Ok(person) => person,
                Err(_) => return Err(EventError::EntityError("Person".to_string())),
            };
            if let Some(birthday) = person.birthday {
                events.push(Event {
                    date: birthday,
                    kind: "Birthday".to_string(),
                    details: EventType::Person(person),
                });
            }
        }

        // TODO handle periodic events
        let mut stmt = match conn
            .prepare("SELECT * FROM reminders WHERE date BETWEEN ?1 AND ?2 AND deleted = 0")
        {
            Ok(stmt) => stmt,
            Err(e) => {
                return Err(EventError::DbError(DbOperationsError::InvalidStatement {
                    sqlite_error: e,
                }))
            }
        };
        let rows = match stmt.query_map(params![today_str, date_limit_str], |row| {
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
            let recurring_type = match RecurringType::get_by_id(&conn, row.get(4)?) {
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
                people,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(EventError::DbError(DbOperationsError::QueryError)),
        };

        for reminder in rows.into_iter() {
            let reminder = match reminder {
                Ok(reminder) => reminder,
                Err(_) => return Err(EventError::EntityError("Reminder".to_string())),
            };
            events.push(Event {
                date: reminder.date,
                kind: "Reminder".to_string(),
                details: EventType::Reminder(reminder),
            });
        }
        Ok(events)
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.details {
            EventType::Person(person) => {
                let mut contact_info_str = String::new();
                for ci in person.contact_info.iter() {
                    contact_info_str.push_str("\n\t");
                    contact_info_str.push_str(ci.contact_info_type.as_ref());
                    contact_info_str.push_str(": ");
                    contact_info_str.push_str(ci.details.as_ref());
                }
                return write!(
                    f,
                    "name: {}\ndate: {}\nkind: {}\ncontact info: {}\n",
                    person.name,
                    &self.date.to_string(),
                    &self.kind,
                    contact_info_str,
                );
            }
            EventType::Reminder(reminder) => {
                return write!(
                    f,
                    "name: {}\ndate: {}\nkind: {}\ndescription: {}\npeople: {}\n",
                    reminder.name,
                    &self.date.to_string(),
                    &self.kind,
                    reminder
                        .description
                        .as_ref()
                        .unwrap_or(&String::from("[Empty]")),
                    reminder
                        .people
                        .iter()
                        .map(|p| p.name.as_str())
                        .collect::<Vec<&str>>()
                        .join(", "),
                );
            }
        };
    }
}

pub trait EventTrait: fmt::Display {}
impl EventTrait for Person {}
impl EventTrait for Reminder {}
impl EventTrait for Event {}
