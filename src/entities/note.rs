use chrono::prelude::*;
use rusqlite::{params, Connection};

use crate::entities::person::Person;
use crate::entities::Entities;

pub static NOTE_TEMPLATE: &str = "Date: {date}
Content: {content}
People: {people}
";

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    id: u64,
    pub date: NaiveDate,
    pub content: String,
    pub people: Vec<Person>,
}

impl Note {
    pub fn new(id: u64, date: NaiveDate, content: String, people: Vec<Person>) -> Note {
        Note {
            id,
            date,
            content,
            people,
        }
    }

    pub fn get_by_person(conn: &Connection, person: String) -> Vec<Note> {
        let person = Person::get_by_name(&conn, &person);
        match person {
            Some(person) => person.notes,
            None => vec![],
        }
    }

    pub fn get_all(conn: &Connection) -> Vec<Note> {
        let mut stmt = conn
            .prepare("SELECT * FROM notes")
            .expect("Invalid SQL statement");

        let rows = stmt
            .query_map([], |row| {
                let note_id = row.get(0).unwrap();
                Ok(Note {
                    id: note_id,
                    date: crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(1).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    content: row.get(2).unwrap(),
                    people: crate::db::db_helpers::get_people_by_note(&conn, note_id),
                })
            })
            .unwrap();

        let mut notes = Vec::new();

        for note in rows.into_iter() {
            notes.push(note.unwrap());
        }

        notes
    }

    pub fn update(
        &mut self,
        conn: &Connection,
        date: Option<String>,
        content: Option<String>,
        people: Vec<String>,
    ) -> &Self {
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

        self.people = Person::get_by_names(&conn, people);

        self
    }

    pub fn parse_from_editor(
        content: &str,
    ) -> Result<(String, String, Vec<String>), crate::editor::ParseError> {
        let mut error = false;
        let mut date: String = String::new();
        let mut note_contents: String = String::new();
        let mut people: Vec<String> = Vec::new();

        let date_prefix = "Date: ";
        let content_prefix = "Content: ";
        let people_prefix = "People: ";

        content.lines().for_each(|line| match line {
            s if s.starts_with(date_prefix) => {
                date = s.trim_start_matches(date_prefix).to_string();
            }
            s if s.starts_with(content_prefix) => {
                note_contents = s.trim_start_matches(content_prefix).to_string();
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

        Ok((date, note_contents, people))
    }
}

impl crate::db::db_interface::DbOperations for Note {
    fn add(&self, conn: &Connection) -> Result<&Note, crate::db::db_interface::DbOperationsError> {
        let date_str = self.date.to_string();

        let mut stmt = conn
            .prepare(
                "INSERT INTO 
                notes (date, content, deleted)
                VALUES (?1, ?2, FALSE)
            ",
            )
            .unwrap();
        match stmt.execute(params![date_str, self.content]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        let id = &conn.last_insert_rowid();

        for person in &self.people {
            let mut stmt = conn
                .prepare(
                    "INSERT INTO people_notes (
                    person_id, 
                    note_id,
                    deleted
                )
                    VALUES (?1, ?2, FALSE)",
                )
                .unwrap();
            match stmt.execute(params![person.id, id]) {
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
        let mut stmt = conn
            .prepare(
                "UPDATE 
                    notes 
                SET
                    deleted = TRUE
                WHERE
                    id = ?1",
            )
            .unwrap();
        match stmt.execute([self.id]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn save(&self, conn: &Connection) -> Result<&Note, crate::db::db_interface::DbOperationsError> {
        let mut stmt = conn
            .prepare(
                "UPDATE
                notes
            SET
                date = ?1,
                content = ?2
            WHERE
                id = ?3",
            )
            .unwrap();
        match stmt.execute(params![self.date.to_string(), self.content, self.id]) {
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
                        people_notes
                    WHERE
                        note_id = ?1 
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
                    let mut stmt = conn
                        .prepare("UPDATE people_notes SET deleted = 1 WHERE id = ?1")
                        .unwrap();
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

            let mut stmt = conn
                .prepare(
                    "INSERT INTO people_notes (
                        person_id,
                        note_id,
                        deleted
                    ) VALUES (?1, ?2, FALSE)",
                )
                .unwrap();
            match stmt.execute(params![person.id, self.id]) {
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
            .prepare("SELECT * FROM notes WHERE id = ?1")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![id]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let note_id = row.get(0).unwrap();
                    Some(Entities::Note(Note {
                        id: note_id,
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(1).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        content: row.get(2).unwrap(),
                        people: crate::db::db_helpers::get_people_by_note(&conn, note_id),
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }
}
