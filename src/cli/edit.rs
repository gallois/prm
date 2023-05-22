use prm::db::db_interface::DbOperations;
use prm::entities::activity::{Activity, ACTIVITY_TEMPLATE};
use prm::entities::note::{Note, NOTE_TEMPLATE};
use prm::entities::person::{Person, PERSON_TEMPLATE};
use prm::entities::reminder::{Reminder, REMINDER_TEMPLATE};
use prm::entities::Entities;
extern crate strfmt;
use rusqlite::Connection;
use std::collections::HashMap;
use strfmt::strfmt;

use crate::cli::add::CliError;
use crate::cli::add::{EditSnafu, EditorParseSnafu, EntitySnafu, NotFoundSnafu};

pub fn person(
    conn: &Connection,
    id: u64,
    name: Option<String>,
    birthday: Option<String>,
    contact_info: Option<String>,
) -> Result<Person, CliError> {
    let name_str: Option<String>;
    let birthday_str: Option<String>;
    let contact_info_str: Option<String>;

    let person = Person::get_by_id(&conn, id);

    match person {
        Some(person) => {
            let mut person = match person {
                Entities::Person(person) => person,
                _ => {
                    return EntitySnafu {
                        entity: "Person".to_string(),
                    }
                    .fail()
                }
            };
            // TODO allow this to be consumed from args like the args below
            let contact_info_field = person
                .contact_info
                .iter()
                .map(|contact_info| {
                    format!(
                        "{}:{}",
                        contact_info.contact_info_type.as_ref().to_lowercase(),
                        contact_info.details
                    )
                })
                .collect::<Vec<String>>()
                .join(",");

            let mut vars = HashMap::new();
            let name_placeholder: String;
            if name.is_some() {
                name_placeholder = name.unwrap();
            } else if !person.name.is_empty() {
                name_placeholder = person.name.clone();
            } else {
                name_placeholder = "".to_string();
            }
            let birthday_placeholder: String;
            if birthday.is_some() {
                birthday_placeholder = birthday.unwrap();
            } else if !person.birthday.is_none() {
                birthday_placeholder = person.birthday.unwrap().to_string();
            } else {
                birthday_placeholder = "".to_string();
            }
            let contact_info_placeholder: String;
            if contact_info.is_some() {
                contact_info_placeholder = contact_info.unwrap();
            } else if !contact_info_field.is_empty() {
                contact_info_placeholder = contact_info_field;
            } else {
                contact_info_placeholder = "".to_string();
            }
            vars.insert("name".to_string(), name_placeholder);
            vars.insert("birthday".to_string(), birthday_placeholder);
            vars.insert("contact_info".to_string(), contact_info_placeholder);

            let edited = edit::edit(strfmt(PERSON_TEMPLATE, &vars).unwrap()).unwrap();
            let (n, b, c) = match Person::parse_from_editor(edited.as_str()) {
                Ok((person, birthday, contact_info)) => (person, birthday, contact_info),
                Err(_) => {
                    return {
                        EditorParseSnafu {
                            entity: "Person".to_string(),
                        }
                        .fail()
                    }
                }
            };
            name_str = Some(n);
            birthday_str = b;
            contact_info_str = Some(c.join(","));

            person.update(name_str, birthday_str, contact_info_str);
            match person.save(&conn) {
                Ok(person) => println!("Updated person: {}", person),
                Err(_) => {
                    return {
                        EditSnafu {
                            entity: "Person".to_string(),
                        }
                        .fail()
                    }
                }
            }
            Ok(person)
        }
        None => {
            println!("Could not find person id {}", id);
            return {
                NotFoundSnafu {
                    entity: "Person".to_string(),
                    id,
                }
                .fail()
            };
        }
    }
}
pub fn activity(
    conn: &Connection,
    id: u64,
    name: Option<String>,
    activity_type: Option<String>,
    date: Option<String>,
    content: Option<String>,
) -> Result<Activity, CliError> {
    let activity = Activity::get_by_id(&conn, id);

    let name_string: String;
    let date_string: String;
    let activity_type_string: String;
    let content_string: String;
    let people: Vec<String>;

    match activity {
        Some(activity) => {
            let mut activity = match activity {
                Entities::Activity(activity) => activity,
                _ => {
                    return {
                        EntitySnafu {
                            entity: "Activity".to_string(),
                        }
                        .fail()
                    }
                }
            };

            let mut vars = HashMap::new();
            let name_placeholder: String;
            if name.clone().is_some() {
                name_placeholder = name.clone().unwrap();
            } else if !activity.name.is_empty() {
                name_placeholder = activity.name.clone();
            } else {
                name_placeholder = "".to_string();
            }
            let date_placeholder: String;
            if date.clone().is_some() {
                date_placeholder = date.clone().unwrap();
            } else if !activity.date.to_string().is_empty() {
                date_placeholder = activity.date.clone().to_string();
            } else {
                date_placeholder = "".to_string();
            }
            let activity_type_placeholder: String;
            if activity_type.clone().is_some() {
                activity_type_placeholder = activity_type.clone().unwrap();
            } else if !activity.activity_type.as_ref().is_empty() {
                activity_type_placeholder =
                    activity.activity_type.as_ref().to_string().to_lowercase();
            } else {
                activity_type_placeholder = "".to_string();
            }
            let content_placeholder: String;
            if content.clone().is_some() {
                content_placeholder = content.clone().unwrap();
            } else if !activity.content.is_empty() {
                content_placeholder = activity.content.clone();
            } else {
                content_placeholder = "".to_string();
            }
            let people_placeholder: String;
            if !activity.people.is_empty() {
                people_placeholder = activity
                    .people
                    .clone()
                    .iter()
                    .map(|p| p.clone().name)
                    .collect::<Vec<String>>()
                    .join(",")
                    .to_string();
            } else {
                people_placeholder = "".to_string();
            }

            vars.insert("name".to_string(), name_placeholder);
            vars.insert("date".to_string(), date_placeholder);
            vars.insert("activity_type".to_string(), activity_type_placeholder);
            vars.insert("content".to_string(), content_placeholder);
            vars.insert("people".to_string(), people_placeholder);

            let edited = edit::edit(strfmt(ACTIVITY_TEMPLATE, &vars).unwrap()).unwrap();
            let (n, d, t, c, p) = match Activity::parse_from_editor(edited.as_str()) {
                Ok((name, date, activity_type, content, people)) => {
                    (name, date, activity_type, content, people)
                }
                Err(_) => {
                    return {
                        EditorParseSnafu {
                            entity: "Activity".to_string(),
                        }
                        .fail()
                    }
                }
            };
            name_string = n;
            date_string = d.unwrap();
            activity_type_string = t.unwrap();
            content_string = c.unwrap();
            people = p;

            activity.update(
                &conn,
                Some(name_string),
                Some(activity_type_string),
                Some(date_string),
                Some(content_string),
                people,
            );
            match activity.save(&conn) {
                Ok(activity) => println!("Updated activity: {:#?}", activity),
                Err(_) => {
                    return {
                        EditSnafu {
                            entity: "Activity".to_string(),
                        }
                    }
                    .fail()
                }
            }
            Ok(activity)
        }
        None => {
            return {
                NotFoundSnafu {
                    entity: "Activity".to_string(),
                    id,
                }
                .fail()
            }
        }
    }
}

pub fn reminder(
    conn: &Connection,
    id: u64,
    name: Option<String>,
    date: Option<String>,
    description: Option<String>,
    recurring: Option<String>,
) -> Result<Reminder, CliError> {
    let reminder = Reminder::get_by_id(&conn, id);

    let name_string: String;
    let date_string: String;
    let recurring_type_string: String;
    let description_string: String;
    let people: Vec<String>;

    match reminder {
        Some(reminder) => {
            let mut reminder = match reminder {
                Entities::Reminder(reminder) => reminder,
                _ => {
                    return {
                        EntitySnafu {
                            entity: "Reminder".to_string(),
                        }
                        .fail()
                    }
                }
            };

            let mut vars = HashMap::new();
            let name_placeholder: String;
            if name.clone().is_some() {
                name_placeholder = name.clone().unwrap();
            } else if !reminder.name.is_empty() {
                name_placeholder = reminder.name.clone();
            } else {
                name_placeholder = "".to_string();
            }
            let date_placeholder: String;
            if date.clone().is_some() {
                date_placeholder = date.clone().unwrap();
            } else if !reminder.date.to_string().is_empty() {
                date_placeholder = reminder.date.clone().to_string();
            } else {
                date_placeholder = "".to_string();
            }
            let description_placeholder: String;
            if description.is_some() {
                description_placeholder = description.clone().unwrap();
            } else if !reminder.description.as_ref().unwrap().is_empty() {
                description_placeholder = reminder.description.clone().unwrap();
            } else {
                description_placeholder = "".to_string();
            }
            let recurring_placeholder: String;
            if recurring.clone().is_some() {
                recurring_placeholder = recurring.clone().unwrap();
            } else if !reminder.recurring.as_ref().is_empty() {
                recurring_placeholder = reminder.recurring.as_ref().to_string().to_lowercase();
            } else {
                recurring_placeholder = "".to_string();
            }
            let people_placeholder: String;
            if !reminder.people.is_empty() {
                people_placeholder = reminder
                    .people
                    .clone()
                    .iter()
                    .map(|p| p.clone().name)
                    .collect::<Vec<String>>()
                    .join(",")
                    .to_string();
            } else {
                people_placeholder = "".to_string();
            }
            vars.insert("date".to_string(), date_placeholder);
            vars.insert("name".to_string(), name_placeholder);
            vars.insert("description".to_string(), description_placeholder);
            vars.insert("recurring_type".to_string(), recurring_placeholder);
            vars.insert("people".to_string(), people_placeholder);

            let edited = edit::edit(strfmt(REMINDER_TEMPLATE, &vars).unwrap()).unwrap();
            let (n, da, r, de, p) = match Reminder::parse_from_editor(edited.as_str()) {
                Ok((name, date, recurring_type, description, people)) => {
                    (name, date, recurring_type, description, people)
                }
                Err(_) => {
                    return {
                        EditorParseSnafu {
                            entity: "Reminder".to_string(),
                        }
                        .fail()
                    }
                }
            };
            name_string = n;
            date_string = da.unwrap();
            recurring_type_string = r.unwrap();
            description_string = de.unwrap();
            people = p;

            reminder.update(
                conn,
                Some(name_string),
                Some(date_string),
                Some(description_string),
                Some(recurring_type_string),
                people,
            );
            match reminder.save(&conn) {
                Ok(reminder) => println!("Updated reminder: {:#?}", reminder),
                Err(_) => {
                    return {
                        EditSnafu {
                            entity: "Reminder".to_string(),
                        }
                        .fail()
                    }
                }
            }
            Ok(reminder)
        }
        None => {
            return {
                NotFoundSnafu {
                    entity: "Reminder".to_string(),
                    id,
                }
                .fail()
            };
        }
    }
}
pub fn note(conn: &Connection, id: u64, date: Option<String>, content: Option<String>) {
    let note = Note::get_by_id(&conn, id);

    let date_string: String;
    let content_string: String;
    let people: Vec<String>;

    match note {
        Some(note) => {
            let mut note = match note {
                Entities::Note(note) => note,
                _ => panic!("not a note"),
            };
            let mut vars = HashMap::new();
            let date_placeholder: String;
            let content_placeholder: String;
            let people_placeholder: String;

            if date.clone().is_some() {
                date_placeholder = date.clone().unwrap();
            } else if !note.date.to_string().is_empty() {
                date_placeholder = note.date.clone().to_string();
            } else {
                date_placeholder = "".to_string();
            }
            if content.clone().is_some() {
                content_placeholder = content.clone().unwrap();
            } else if !note.content.is_empty() {
                content_placeholder = note.content.clone();
            } else {
                content_placeholder = "".to_string();
            }
            if !note.people.is_empty() {
                people_placeholder = note
                    .people
                    .clone()
                    .iter()
                    .map(|p| p.clone().name)
                    .collect::<Vec<String>>()
                    .join(",")
                    .to_string();
            } else {
                people_placeholder = "".to_string();
            }

            vars.insert("date".to_string(), date_placeholder);
            vars.insert("content".to_string(), content_placeholder);
            vars.insert("people".to_string(), people_placeholder);

            let edited = edit::edit(strfmt(NOTE_TEMPLATE, &vars).unwrap()).unwrap();
            let (d, c, p) = match Note::parse_from_editor(edited.as_str()) {
                Ok((date, content, people)) => (date, content, people),
                Err(_) => panic!("Error parsing note"),
            };

            date_string = d;
            content_string = c;
            people = p;

            note.update(conn, Some(date_string), Some(content_string), people);
            note.save(&conn)
                .expect(format!("Failed to update note: {:#?}", note).as_str());
            println!("Updated note: {:#?}", note);
        }
        None => {
            println!("Could not find note id {}", id);
            return;
        }
    }
}
