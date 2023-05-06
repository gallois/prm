use prm::db::db_interface::DbOperations;
use prm::entities::{Activity, Entities, Note, Person, Reminder};
use prm::entities::{ACTIVITY_TEMPLATE, NOTE_TEMPLATE, PERSON_TEMPLATE, REMINDER_TEMPLATE};
extern crate strfmt;
use rusqlite::Connection;
use std::collections::HashMap;
use strfmt::strfmt;

pub fn person(
    conn: &Connection,
    id: u64,
    name: Option<String>,
    birthday: Option<String>,
    contact_info: Option<String>,
) {
    let name_str: Option<String>;
    let birthday_str: Option<String>;
    let contact_info_str: Option<String>;

    let person = Person::get_by_id(&conn, id);

    match person {
        Some(person) => {
            let mut person = match person {
                Entities::Person(person) => person,
                _ => panic!("not a person"),
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
                Err(_) => panic!("Error parsing person"),
            };
            name_str = Some(n);
            birthday_str = b;
            contact_info_str = Some(c.join(","));

            person.update(name_str, birthday_str, contact_info_str);
            person
                .save(&conn)
                .expect(format!("Failed to update person: {}", person).as_str());
            println!("Updated person: {}", person);
        }
        None => {
            println!("Could not find person id {}", id);
            return;
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
) {
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
                _ => panic!("not an activity"),
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
                Err(_) => panic!("Error parsing activity"),
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
            activity
                .save(&conn)
                .expect(format!("Failed to update activity: {:#?}", activity).as_str());
            println!("Updated activity: {:#?}", activity);
        }
        None => {
            println!("Could not find activity id {}", id);
            return;
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
) {
    let reminder = Reminder::get_by_id(&conn, id);

    let name_string: String;
    let date_string: String;
    let recurring_type_string: String;
    let description_string: String;
    let people: Vec<String>;

    // TODO include people when editing
    match reminder {
        Some(reminder) => {
            let mut reminder = match reminder {
                Entities::Reminder(reminder) => reminder,
                _ => panic!("not a reminder"),
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
                Err(_) => panic!("Error parsing reminder"),
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
            reminder
                .save(&conn)
                .expect(format!("Failed to update reminder: {:#?}", reminder).as_str());
            println!("Updated reminder: {:#?}", reminder);
        }
        None => {
            println!("Could not find reminder id {}", id);
            return;
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
            // if [date.clone(), content.clone()].iter().all(Option::is_none) {
            //     println!("You must set at least one of `date` or `content`");
            // }
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
