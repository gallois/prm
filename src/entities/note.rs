use std::fmt;

use chrono::prelude::*;
use rusqlite::params;

use crate::db_interface::{DbOperations, DbOperationsError};
use crate::entities::person::Person;
use crate::entities::Entities;
use crate::{CliError, DateParseSnafu, RecordParseSnafu};
use rusqlite::Connection;

pub static NOTE_TEMPLATE: &str = "Date: {date}
Content: {content}
People: {people}
";

use super::Entity;

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub id: u64,
    pub date: NaiveDate,
    pub content: String,
    pub people: Vec<Person>,
}

impl Entity for Note {
    fn get_id(&self) -> u64 {
        self.id
    }
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

    pub fn get(
        conn: &Connection,
        person: Option<String>,
        content: Option<String>,
    ) -> Result<Vec<Note>, DbOperationsError> {
        let mut notes: Vec<Note> = vec![];
        if let Some(person) = person {
            notes = Self::get_by_person(conn, person)?;
            return Ok(notes);
        }
        if let Some(content) = content {
            notes = Self::get_by_content(conn, content)?;
        }
        Ok(notes)
    }

    pub fn get_by_content(
        conn: &Connection,
        content: String,
    ) -> Result<Vec<Note>, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "SELECT 
                * 
            FROM 
                notes
            WHERE
                content LIKE '%' || ?1 || '%'
                AND deleted = 0
            COLLATE NOCASE",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };

        let mut notes: Vec<Note> = vec![];

        let mut rows = match stmt.query([content]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
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
                                })
                            }
                        };
                        let date = row.get::<usize, String>(1);
                        let date =
                            crate::helpers::parse_from_str_ymd(date.unwrap_or_default().as_str())
                                .unwrap_or_default();
                        let content = match row.get(2) {
                            Ok(content) => content,
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        };
                        let people = crate::db::db_helpers::get_people_by_note(conn, id)?;
                        notes.push(Note::new(id, date, content, people))
                    }
                    None => return Ok(notes),
                },
                Err(_) => return Err(DbOperationsError::GenericError),
            }
        }
    }

    fn get_by_person(conn: &Connection, person: String) -> Result<Vec<Note>, DbOperationsError> {
        let person = Person::get_by_name(conn, Some(person), None);
        match person {
            Ok(person) => {
                if person.len() > 1 {
                    return Err(DbOperationsError::UnexpectedMultipleEntries);
                }
                let notes = person[0].notes.clone();
                Ok(notes)
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_all(conn: &Connection) -> Result<Vec<Note>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM notes WHERE deleted = 0") {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };

        let rows = match stmt.query_map([], |row| {
            let note_id = row.get(0)?;
            let people = match crate::db::db_helpers::get_people_by_note(conn, note_id) {
                Ok(people) => people,
                Err(e) => {
                    let sqlite_error = match e {
                        DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                        other => panic!("Unexpected error type: {:#?}", other),
                    };
                    return Err(sqlite_error);
                }
            };
            Ok(Note {
                id: note_id,
                date: crate::helpers::parse_from_str_ymd(
                    row.get::<usize, String>(1).unwrap_or_default().as_str(),
                )
                .unwrap_or_default(),
                content: row.get(2)?,
                people,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut notes = Vec::new();

        for note in rows.into_iter() {
            let note = match note {
                Ok(note) => note,
                Err(e) => {
                    return Err(DbOperationsError::RecordError {
                        sqlite_error: Some(e),
                        strum_error: None,
                    })
                }
            };
            notes.push(note);
        }

        Ok(notes)
    }

    pub fn update(
        &mut self,
        conn: &Connection,
        date: Option<String>,
        content: Option<String>,
        people: Vec<String>,
    ) -> Result<&Self, CliError> {
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
            };
        }

        if let Some(content) = content {
            self.content = content;
        }

        self.people = match Person::get_by_names(conn, people) {
            Ok(people) => people,
            Err(_) => {
                return RecordParseSnafu {
                    record: "people".to_string(),
                }
                .fail()
            }
        };

        Ok(self)
    }

    pub fn parse_from_editor(content: &str) -> Result<(String, String, Vec<String>), CliError> {
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
                people = people_str.split(',').map(|x| x.to_string()).collect();
            }
            _ => error = true,
        });

        if error {
            return Err(CliError::FormatError);
        }

        Ok((date, note_contents, people))
    }
}

impl DbOperations for Note {
    fn add(&self, conn: &Connection) -> Result<&Note, DbOperationsError> {
        let date_str = self.date.to_string();

        let mut stmt = match conn.prepare(
            "INSERT INTO 
                notes (date, content, deleted)
                VALUES (?1, ?2, FALSE)
            ",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };

        match stmt.execute(params![date_str, self.content]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        let id = &conn.last_insert_rowid();

        for person in &self.people {
            let mut stmt = match conn.prepare(
                "INSERT INTO people_notes (
                    person_id, 
                    note_id,
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
                    notes 
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

    fn save(&self, conn: &Connection) -> Result<&Note, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "UPDATE
                notes
            SET
                date = ?1,
                content = ?2
            WHERE
                id = ?3",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
        };

        match stmt.execute(params![self.date.to_string(), self.content, self.id]) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(DbOperationsError::QueryError),
        }

        for person in self.people.iter() {
            let mut stmt = match conn.prepare(
                "SELECT 
                        id
                    FROM
                        people_notes
                    WHERE
                        note_id = ?1 
                        AND person_id = ?2
                        AND deleted = 0",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
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

            if !results.is_empty() {
                for id in results {
                    let mut stmt =
                        match conn.prepare("UPDATE people_notes SET deleted = 1 WHERE id = ?1") {
                            Ok(stmt) => stmt,
                            Err(e) => {
                                return Err(DbOperationsError::InvalidStatement { sqlite_error: e })
                            }
                        };

                    match stmt.execute(params![id]) {
                        Ok(updated) => {
                            println!("[DEBUG] {} rows were updated", updated);
                        }
                        Err(_) => return Err(DbOperationsError::QueryError),
                    }
                }
            }

            let mut stmt = match conn.prepare(
                "INSERT INTO people_notes (
                        person_id,
                        note_id,
                        deleted
                    ) VALUES (?1, ?2, FALSE)",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            match stmt.execute(params![person.id, self.id]) {
                Ok(updated) => {
                    println!("[DEBUG] {} rows were updated", updated);
                }
                Err(_) => return Err(DbOperationsError::QueryError),
            }
        }

        Ok(self)
    }

    fn get_by_id(conn: &Connection, id: u64) -> Result<Option<Entities>, DbOperationsError> {
        let mut stmt = match conn.prepare("SELECT * FROM notes WHERE id = ?1 AND deleted = 0") {
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
                    let note_id = match row.get(0) {
                        Ok(note_id) => note_id,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    let people = crate::db::db_helpers::get_people_by_note(conn, note_id)?;
                    let content = match row.get(2) {
                        Ok(content) => content,
                        Err(e) => {
                            return Err(DbOperationsError::RecordError {
                                sqlite_error: Some(e),
                                strum_error: None,
                            })
                        }
                    };
                    Ok(Some(Entities::Note(Note {
                        id: note_id,
                        date: crate::helpers::parse_from_str_ymd(
                            row.get::<usize, String>(1).unwrap_or_default().as_str(),
                        )
                        .unwrap_or_default(),
                        content,
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
        // TODO implement get all
        todo!()
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut people_str = String::new();
        for person in self.people.iter() {
            people_str.push_str("\n\t");
            people_str.push_str(format!("name: {}", person.name).as_ref());
        }
        write!(
            f,
            "note id: {}\ncontent: {}\ndate: {}\npeople:{}\n",
            &self.id,
            &self.content,
            &self.date.to_string(),
            people_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let id = 1;
        let date = crate::helpers::parse_from_str_ymd("2020-01-01").unwrap();
        let content = String::from("book");
        let people: Vec<Person> = vec![];

        let note = Note::new(id, date, content.clone(), people.clone());

        assert_eq!(
            Note {
                id,
                date,
                content,
                people,
            },
            note
        );
    }
}
