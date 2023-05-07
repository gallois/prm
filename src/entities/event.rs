use chrono::prelude::*;
use rusqlite::{params, Connection};
use std::{convert::AsRef, fmt};

use crate::entities::person::Person;
use crate::entities::reminder::{RecurringType, Reminder};

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
    pub fn get_all(conn: &Connection, days: u64) -> Vec<Event> {
        let mut events: Vec<Event> = vec![];
        let today = chrono::Local::now().naive_local();
        let today_str = format!("{}", today.format("%Y-%m-%d"));
        let date_limit = today.checked_add_days(chrono::Days::new(days)).unwrap();
        let date_limit_str = format!("{}", date_limit.format("%Y-%m-%d"));

        let mut stmt = conn
            .prepare(
                "SELECT
                    *,
                    strftime('%j', birthday) - strftime('%j', 'now') AS days_remaining
                FROM
                    people
                WHERE ?1 >= CASE
                    WHEN days_remaining >= 0 THEN days_remaining
                    ELSE days_remaining + strftime('%j', strftime('%Y-12-31', 'now'))
                    END
                ",
            )
            .expect("Invalid SQL statement");

        let rows = stmt
            .query_map(params![days], |row| {
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
        for person in rows.into_iter() {
            let person = person.unwrap();
            if let Some(birthday) = person.birthday {
                events.push(Event {
                    date: birthday,
                    kind: "Birthday".to_string(),
                    details: EventType::Person(person),
                });
            }
        }

        // TODO handle periodic events
        let mut stmt = conn
            .prepare("SELECT * FROM reminders WHERE date BETWEEN ?1 AND ?2")
            .expect("Invalid SQL statement");
        let rows = stmt
            .query_map(params![today_str, date_limit_str], |row| {
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
        for reminder in rows.into_iter() {
            let reminder = reminder.unwrap();
            events.push(Event {
                date: reminder.date,
                kind: "Reminder".to_string(),
                details: EventType::Reminder(reminder),
            });
        }
        events
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
