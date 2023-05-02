pub mod db;

pub use crate::db::{db_helpers, db_interface};

use chrono::prelude::*;
use rusqlite::{params, params_from_iter, Connection};
use std::{convert::AsRef, fmt, str::FromStr};
use strum_macros::{AsRefStr, EnumString};

pub enum ParseError {
    FieldError,
    FormatError,
}

pub static PERSON_TEMPLATE: &str = "Name: {name}
Birthday: {birthday}
Contact Info: {contact_info}
";

pub static ACTIVITY_TEMPLATE: &str = "Name: {name}
Date: {date}
Activity Type: {activity_type}
Content: {content}
People: {people}
";

pub static REMINDER_TEMPLATE: &str = "Name: {name}
Date: {date}
Recurring: {recurring_type}
Description: {description}
People: {people}
";

pub static NOTE_TEMPLATE: &str = "Content: {content}
People: {people}
";

pub mod helpers {
    // Helper function to return a comma-separated sequence of `?`.
    // - `repeat_vars(0) => panic!(...)`
    // - `repeat_vars(1) => "?"`
    // - `repeat_vars(2) => "?,?"`
    // - `repeat_vars(3) => "?,?,?"`
    // - ...
    pub fn repeat_vars(count: usize) -> String {
        assert_ne!(count, 0);
        let mut s = "?,".repeat(count);
        // Remove trailing comma
        s.pop();
        s
    }

    pub fn parse_from_str_ymd(date: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
    }

    pub fn parse_from_str_md(date: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
        parse_from_str_ymd(format!("1-{}", date).as_ref())
    }

    pub fn unwrap_arg_or_empty_string(arg: Option<String>) -> String {
        arg.unwrap_or("".to_string())
    }
}

pub mod cli {
    pub mod add {
        use crate::db::db_interface::DbOperations;
        use crate::{
            helpers, Activity, ActivityType, Connection, ContactInfo, ContactInfoType, Note,
            Person, RecurringType, Reminder, ACTIVITY_TEMPLATE, NOTE_TEMPLATE, PERSON_TEMPLATE,
            REMINDER_TEMPLATE,
        };
        use chrono::prelude::*;
        use chrono::NaiveDate;
        use edit;

        extern crate strfmt;
        use std::collections::HashMap;
        use strfmt::strfmt;

        pub fn person(
            conn: &Connection,
            name: Option<String>,
            birthday: Option<String>,
            contact_info: Option<Vec<String>>,
        ) {
            let mut name_str: String = String::new();
            let mut birthday_str: Option<String> = None;
            let mut contact_info_vec: Vec<String> = vec![];
            let mut editor = false;
            if name == None {
                editor = true;

                let mut vars = HashMap::new();
                vars.insert(
                    "name".to_string(),
                    helpers::unwrap_arg_or_empty_string(name.clone()),
                );
                vars.insert(
                    "birthday".to_string(),
                    helpers::unwrap_arg_or_empty_string(birthday.clone()),
                );
                vars.insert(
                    "contact_info".to_string(),
                    contact_info.clone().unwrap_or_default().join(","),
                );

                let edited = edit::edit(strfmt(PERSON_TEMPLATE, &vars).unwrap()).unwrap();
                let (n, b, c) = match Person::parse_from_editor(edited.as_str()) {
                    Ok((person, birthday, contact_info)) => (person, birthday, contact_info),
                    Err(_) => panic!("Error parsing person"),
                };
                name_str = n;
                birthday_str = b;
                contact_info_vec = c;
            }

            if !editor {
                name_str = name.unwrap();
            }
            let mut birthday_obj: Option<NaiveDate> = None;
            if !editor {
                if let Some(bday) = birthday {
                    birthday_str = Some(bday);
                }
            }

            if let Some(birthday_str) = birthday_str {
                match helpers::parse_from_str_ymd(&birthday_str) {
                    // TODO proper error handling and messaging
                    Ok(date) => birthday_obj = Some(date),
                    Err(_) => match helpers::parse_from_str_md(&birthday_str) {
                        Ok(date) => birthday_obj = Some(date),
                        Err(error) => panic!("Error parsing birthday: {}", error),
                    },
                }
            }

            let mut contact_info_splits: Vec<Vec<String>> = vec![];
            let mut contact_info_types: Vec<ContactInfoType> = vec![];

            match contact_info {
                Some(mut contact_info_vec) => {
                    if !editor {
                        ContactInfo::populate_splits(
                            &mut contact_info_splits,
                            &mut contact_info_vec,
                        );
                    }
                }
                None => {
                    if editor {
                        ContactInfo::populate_splits(
                            &mut contact_info_splits,
                            &mut contact_info_vec,
                        );
                    }
                }
            }

            if contact_info_splits.len() > 0 {
                contact_info_splits
                    .into_iter()
                    .for_each(|contact_info_split| {
                        match contact_info_split[0].as_str() {
                            "phone" => contact_info_types
                                .push(ContactInfoType::Phone(contact_info_split[1].clone())),
                            "whatsapp" => contact_info_types
                                .push(ContactInfoType::WhatsApp(contact_info_split[1].clone())),
                            "email" => contact_info_types
                                .push(ContactInfoType::Email(contact_info_split[1].clone())),
                            // TODO proper error handling and messaging
                            _ => panic!("Unknown contact info type"),
                        }
                    });
            }

            let mut contact_info: Vec<ContactInfo> = Vec::new();
            if contact_info_types.len() > 0 {
                contact_info_types
                    .into_iter()
                    .for_each(|contact_info_type| {
                        contact_info.push(ContactInfo::new(0, 0, contact_info_type));
                    });
            }

            assert_eq!(name_str.is_empty(), false, "Name cannot be empty");
            let person = Person::new(0, name_str, birthday_obj, contact_info);
            match person.add(&conn) {
                Ok(_) => println!("{} added successfully", person),
                Err(_) => panic!("Error while adding {}", person),
            };
        }
        pub fn activity(
            conn: &Connection,
            name: Option<String>,
            activity_type: Option<String>,
            date: Option<String>,
            content: Option<String>,
            mut people: Vec<String>,
        ) {
            let name_string: String;
            let date_string: String;
            let activity_type_string: String;
            let content_string: String;

            if name == None {
                let mut vars = HashMap::new();
                vars.insert(
                    "name".to_string(),
                    helpers::unwrap_arg_or_empty_string(name.clone()),
                );
                vars.insert(
                    "date".to_string(),
                    helpers::unwrap_arg_or_empty_string(date.clone()),
                );
                vars.insert(
                    "activity_type".to_string(),
                    helpers::unwrap_arg_or_empty_string(activity_type.clone()),
                );
                vars.insert(
                    "content".to_string(),
                    helpers::unwrap_arg_or_empty_string(content.clone()),
                );
                vars.insert(
                    "people".to_string(),
                    if people.is_empty() {
                        "".to_string()
                    } else {
                        people.clone().join(",")
                    },
                );

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
            } else {
                if [activity_type.clone(), date.clone(), content.clone()]
                    .iter()
                    .any(Option::is_none)
                {
                    // FIXME opening the editor here is a better behaviour
                    println!("if `name` isn't set, all of `activity_type`, `date`, and `content` must be set");
                    return;
                }
                name_string = name.unwrap();
                activity_type_string = activity_type.unwrap();
                date_string = date.unwrap();
                content_string = content.unwrap();
            }

            let activity_type = match activity_type_string.as_str() {
                "phone" => ActivityType::Phone,
                "in_person" => ActivityType::InPerson,
                "online" => ActivityType::Online,
                // TODO proper error handling and messaging
                _ => panic!("Unknown reminder type"),
            };

            let date_obj = match helpers::parse_from_str_ymd(date_string.as_str()) {
                Ok(date) => date,
                Err(error) => panic!("Error parsing date: {}", error),
            };

            let people = Person::get_by_names(&conn, people);

            let activity = Activity::new(
                0,
                name_string,
                activity_type,
                date_obj,
                content_string,
                people,
            );
            match activity.add(&conn) {
                Ok(_) => println!("{:#?} added successfully", activity),
                Err(_) => panic!("Error while adding {:#?}", activity),
            };
        }

        pub fn reminder(
            conn: &Connection,
            name: Option<String>,
            date: Option<String>,
            recurring: Option<String>,
            description: Option<String>,
            mut people: Vec<String>,
        ) {
            let mut name_string: String = String::new();
            let mut date_string: String = String::new();
            let mut recurring_type_string: String = String::new();
            let mut description_string: String = String::new();

            let mut editor = false;
            if name == None {
                editor = true;

                let mut vars = HashMap::new();
                vars.insert(
                    "name".to_string(),
                    helpers::unwrap_arg_or_empty_string(name.clone()),
                );
                vars.insert(
                    "date".to_string(),
                    helpers::unwrap_arg_or_empty_string(date.clone()),
                );
                vars.insert(
                    "recurring_type".to_string(),
                    helpers::unwrap_arg_or_empty_string(recurring.clone()),
                );
                vars.insert(
                    "description".to_string(),
                    helpers::unwrap_arg_or_empty_string(description.clone()),
                );
                vars.insert(
                    "people".to_string(),
                    if people.is_empty() {
                        "".to_string()
                    } else {
                        people.clone().join(",")
                    },
                );

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
            }

            if !editor {
                name_string = name.unwrap();
                date_string = date.unwrap();
                recurring_type_string = recurring.unwrap();
                description_string = description.unwrap_or("".to_string());
            }

            let recurring_type = match recurring_type_string {
                recurring_type_str => match recurring_type_str.as_str() {
                    "daily" => RecurringType::Daily,
                    "weekly" => RecurringType::Weekly,
                    "fortnightly" => RecurringType::Fortnightly,
                    "monthly" => RecurringType::Monthly,
                    "quarterly" => RecurringType::Quarterly,
                    "biannual" => RecurringType::Biannual,
                    "yearly" => RecurringType::Yearly,
                    "onetime" => RecurringType::OneTime,
                    _ => panic!("Unknown recurring pattern"),
                },
            };

            let date_obj = match helpers::parse_from_str_ymd(date_string.as_str()) {
                Ok(date) => date,
                Err(error) => panic!("Error parsing date: {}", error),
            };

            let people = Person::get_by_names(&conn, people);

            let reminder = Reminder::new(
                0,
                name_string,
                date_obj,
                Some(description_string),
                recurring_type,
                people,
            );
            println!("Reminder: {:#?}", reminder);
            match reminder.add(&conn) {
                Ok(_) => println!("{:#?} added successfully", reminder),
                Err(_) => panic!("Error while adding {:#?}", reminder),
            };
        }

        pub fn note(conn: &Connection, content: Option<String>, people: Vec<String>) {
            let mut content_string: String = String::new();
            let mut people_vec: Vec<Person> = Vec::new();

            let mut editor = false;
            if content == None {
                editor = true;

                let mut vars = HashMap::new();
                vars.insert(
                    "content".to_string(),
                    helpers::unwrap_arg_or_empty_string(content.clone()),
                );
                vars.insert("people".to_string(), people.clone().join(","));

                let edited = edit::edit(strfmt(NOTE_TEMPLATE, &vars).unwrap()).unwrap();
                let (c, p) = match Note::parse_from_editor(edited.as_str()) {
                    Ok((c, p)) => (c, p),
                    Err(_) => panic!("Error parsing note"),
                };
                content_string = c;
                people_vec = Person::get_by_names(&conn, p);
            }

            if !editor {
                content_string = content.unwrap();
                people_vec = Person::get_by_names(&conn, people);
            }
            let date = Utc::now().date_naive();

            let note = Note::new(0, date, content_string, people_vec);
            println!("Note: {:#?}", note);
            match note.add(&conn) {
                Ok(_) => println!("{:#?} added successfully", note),
                Err(_) => panic!("Error while adding {:#?}", note),
            };
        }
    }

    pub mod edit {
        use crate::db::db_interface::DbOperations;
        use crate::{
            Activity, Connection, Entities, Note, Person, RecurringType, Reminder,
            ACTIVITY_TEMPLATE, PERSON_TEMPLATE, REMINDER_TEMPLATE,
        };
        extern crate strfmt;
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
                    vars.insert("name".to_string(), name_placeholder);
                    vars.insert("birthday".to_string(), birthday_placeholder);
                    vars.insert("contact_info".to_string(), contact_info_field);

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

            let mut name_string: String = String::new();
            let mut date_string: String = String::new();
            let mut recurring_type_string: String = String::new();
            let mut description_string: String = String::new();
            let mut people_string: String = String::new();
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
                        recurring_placeholder =
                            reminder.recurring.as_ref().to_string().to_lowercase();
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

            match note {
                Some(note) => {
                    if [date.clone(), content.clone()].iter().all(Option::is_none) {
                        println!("You must set at least one of `date` or `content`");
                    }
                    if let Entities::Note(mut note) = note {
                        note.update(date, content);
                        note.save(&conn)
                            .expect(format!("Failed to update note: {:#?}", note).as_str());
                        println!("Updated note: {:#?}", note);
                    }
                }
                None => {
                    println!("Could not find note id {}", id);
                    return;
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Entities {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
    Note(Note),
}

#[derive(Debug, Clone)]
pub struct Person {
    id: u64,
    name: String,
    birthday: Option<NaiveDate>,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
    notes: Vec<Note>,
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
    ) -> Result<(String, Option<String>, Vec<String>), crate::ParseError> {
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
            return Err(crate::ParseError::FormatError);
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

    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
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

#[derive(Debug, Clone)]
pub struct Activity {
    id: u64,
    name: String,
    activity_type: ActivityType,
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
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
                        activity_type: crate::ActivityType::get_by_id(&conn, row.get(2).unwrap())
                            .unwrap(),
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
                    activity_type: crate::ActivityType::get_by_id(&conn, row.get(2).unwrap())
                        .unwrap(),
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

    // TODO might be a good idea to edit people
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

        let people = crate::Person::get_by_names(&conn, people);
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
        crate::ParseError,
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
            return Err(crate::ParseError::FormatError);
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

    fn remove(&self, conn: &crate::Connection) -> Result<&Self, db_interface::DbOperationsError> {
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

    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
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
                        activity_type: crate::ActivityType::get_by_id(&conn, row.get(2).unwrap())
                            .unwrap(),
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

#[derive(Debug, AsRefStr, EnumString, Clone)]
pub enum ActivityType {
    Phone,
    InPerson,
    Online,
}

impl ActivityType {
    fn get_by_id(conn: &Connection, id: u64) -> Option<ActivityType> {
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

#[derive(Debug, Clone)]
pub struct Reminder {
    id: u64,
    name: String,
    date: NaiveDate,
    description: Option<String>,
    recurring: RecurringType,
    people: Vec<Person>,
}

impl Reminder {
    pub fn new(
        id: u64,
        name: String,
        date: NaiveDate,
        description: Option<String>,
        recurring: RecurringType,
        people: Vec<Person>,
    ) -> Reminder {
        Reminder {
            id,
            name,
            date,
            description,
            recurring,
            people,
        }
    }

    // TODO remove duplication between different entities
    pub fn get_by_name(conn: &Connection, name: &str) -> Option<Reminder> {
        let mut stmt = conn
            .prepare("SELECT * FROM reminders WHERE name = ?1 COLLATE NOCASE")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![name]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let reminder_id = row.get(0).unwrap();
                    Some(Reminder {
                        id: reminder_id,
                        name: row.get(1).unwrap(),
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        description: row.get(3).unwrap(),
                        recurring: crate::RecurringType::get_by_id(&conn, row.get(4).unwrap())
                            .unwrap(),
                        people: crate::db::db_helpers::get_people_by_reminder(&conn, reminder_id),
                    })
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }

    pub fn get_all(conn: &Connection, include_past: bool) -> Vec<Reminder> {
        let sql: String;
        let base_sql = "SELECT * FROM reminders";
        if include_past {
            sql = format!("{}", base_sql);
        } else {
            sql = format!("{} WHERE date > DATE()", base_sql);
        }

        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");
        let rows = stmt
            .query_map([], |row| {
                let reminder_id = row.get(0).unwrap();
                Ok(Reminder {
                    id: reminder_id,
                    name: row.get(1).unwrap(),
                    date: crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    description: row.get(3).unwrap(),
                    recurring: crate::RecurringType::get_by_id(&conn, row.get(4).unwrap()).unwrap(),
                    people: crate::db::db_helpers::get_people_by_reminder(&conn, reminder_id),
                })
            })
            .unwrap();

        let mut reminders = Vec::new();

        for reminder in rows.into_iter() {
            reminders.push(reminder.unwrap());
        }

        reminders
    }

    pub fn update(
        &mut self,
        conn: &Connection,
        name: Option<String>,
        date: Option<String>,
        description: Option<String>,
        recurring: Option<String>,
        people: Vec<String>,
    ) -> &Self {
        if let Some(name) = name {
            self.name = name;
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

        // TODO we need a way to unset description
        if let Some(description) = description {
            self.description = Some(description);
        }

        // TODO remove duplication between here and main.rs
        let recurring_type = match recurring {
            Some(recurring_type_str) => match recurring_type_str.as_str() {
                "daily" => Some(RecurringType::Daily),
                "weekly" => Some(RecurringType::Weekly),
                "fortnightly" => Some(RecurringType::Fortnightly),
                "monthly" => Some(RecurringType::Monthly),
                "quarterly" => Some(RecurringType::Quarterly),
                "biannual" => Some(RecurringType::Biannual),
                "yearly" => Some(RecurringType::Yearly),
                "onetime" => Some(RecurringType::OneTime),
                _ => panic!("Unknown recurring pattern"),
            },
            None => Some(RecurringType::OneTime),
        };

        if let Some(recurring_type) = recurring_type {
            self.recurring = recurring_type;
        }

        let people = crate::Person::get_by_names(&conn, people);
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
        crate::ParseError,
    > {
        let mut error = false;
        let mut name: String = String::new();
        let mut date: Option<String> = None;
        let mut recurring_type: Option<String> = None;
        let mut description: Option<String> = None;
        let mut people = Vec::new();

        let name_prefix = "Name: ";
        let date_prefix = "Date: ";
        let recurring_type_prefix = "Recurring: ";
        let description_prefix = "Description: ";
        let people_prefix = "People: ";

        content.lines().for_each(|line| match line {
            s if s.starts_with(name_prefix) => {
                name = s.trim_start_matches(name_prefix).to_string();
            }
            s if s.starts_with(date_prefix) => {
                date = Some(s.trim_start_matches(date_prefix).to_string());
            }
            s if s.starts_with(recurring_type_prefix) => {
                recurring_type = Some(s.trim_start_matches(recurring_type_prefix).to_string());
            }
            s if s.starts_with(description_prefix) => {
                description = Some(s.trim_start_matches(description_prefix).to_string());
            }
            s if s.starts_with(people_prefix) => {
                let people_str = s.trim_start_matches(people_prefix);
                people = people_str.split(",").map(|x| x.to_string()).collect();
            }
            // FIXME
            _ => error = true,
        });

        if error {
            return Err(crate::ParseError::FormatError);
        }

        Ok((name, date, recurring_type, description, people))
    }
}

impl crate::db::db_interface::DbOperations for Reminder {
    fn add(
        &self,
        conn: &Connection,
    ) -> Result<&Reminder, crate::db::db_interface::DbOperationsError> {
        let mut stmt = conn
            .prepare("SELECT id FROM reminders WHERE name = ?")
            .unwrap();
        let mut rows = stmt.query(params![self.name]).unwrap();
        let mut ids: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            ids.push(row.get(0).unwrap());
        }

        if ids.len() > 0 {
            return Err(crate::db::db_interface::DbOperationsError::DuplicateEntry);
        }

        let recurring_str = &self.recurring.as_ref();

        let date_str = self.date.to_string();

        // TODO error handling
        let mut stmt = conn
            .prepare("SELECT id FROM recurring_types WHERE type = ?")
            .unwrap();
        let mut rows = stmt.query(params![recurring_str]).unwrap();
        let mut types: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            types.push(row.get(0).unwrap());
        }

        match conn.execute(
            "INSERT INTO 
                reminders (name, date, recurring, description, deleted)
                VALUES (?1, ?2, ?3, ?4, FALSE)
            ",
            params![self.name, date_str, types[0], self.description],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        let id = conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_reminders (
                    person_id, 
                    reminder_id,
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

    fn remove(&self, conn: &crate::Connection) -> Result<&Self, db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE 
                    reminders 
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
    ) -> Result<&Reminder, crate::db::db_interface::DbOperationsError> {
        let recurring_str = &self.recurring.as_ref();

        let date_str = self.date.to_string();

        // TODO error handling
        let mut stmt = conn
            .prepare("SELECT id FROM recurring_types WHERE type = ?")
            .unwrap();
        let mut rows = stmt.query(params![recurring_str]).unwrap();
        let mut types: Vec<u32> = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            types.push(row.get(0).unwrap());
        }

        match conn.execute(
            "UPDATE
                reminders 
            SET
                name = ?1,
                date = ?2,
                recurring = ?3,
                description = ?4
            WHERE
                id = ?5
            ",
            params![self.name, date_str, types[0], self.description, self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        // TODO allow for changing people
        Ok(self)
    }

    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
        let mut stmt = conn
            .prepare("SELECT * FROM reminders WHERE id = ?1")
            .expect("Invalid SQL statement");
        let mut rows = stmt.query(params![id]).unwrap();
        match rows.next() {
            Ok(row) => match row {
                Some(row) => {
                    let reminder_id = row.get(0).unwrap();
                    Some(Entities::Reminder(Reminder {
                        id: reminder_id,
                        name: row.get(1).unwrap(),
                        date: crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                        description: row.get(3).unwrap(),
                        recurring: crate::RecurringType::get_by_id(&conn, row.get(4).unwrap())
                            .unwrap(),
                        people: crate::db::db_helpers::get_people_by_reminder(&conn, reminder_id),
                    }))
                }
                None => return None,
            },
            Err(_) => return None,
        }
    }
}

impl fmt::Display for Reminder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description_str = match &self.description {
            Some(description) => description.as_ref(),
            None => "",
        };
        let recurring_type_str = &self.recurring.as_ref();
        let mut people_str = String::new();
        for person in self.people.iter() {
            people_str.push_str("\n\t");
            people_str.push_str(format!("name: {}\n\t", person.name).as_ref());
        }
        write!(
            f,
            "reminder id: {}\nname: {}\ndate: {}\ndescription: {}\nrecurring type: {}\npeople:{}\n",
            &self.id,
            &self.name,
            &self.date.to_string(),
            description_str,
            recurring_type_str,
            people_str
        )
    }
}

#[derive(Debug, AsRefStr, EnumString, Clone)]
pub enum RecurringType {
    OneTime,
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Biannual,
    Yearly,
}

impl RecurringType {
    fn get_by_id(conn: &Connection, id: u64) -> Option<RecurringType> {
        let mut stmt = conn
            .prepare("SELECT type FROM recurring_types WHERE id = ?")
            .unwrap();
        let mut rows = stmt.query(params![id]).unwrap();

        match rows.next() {
            Ok(row) => match row {
                Some(row) => Some(
                    RecurringType::from_str(row.get::<usize, String>(0).unwrap().as_str()).unwrap(),
                ),
                None => None,
            },
            Err(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContactInfo {
    id: u64,
    person_id: u64,
    pub contact_info_type: ContactInfoType,
    details: String,
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

#[derive(Debug, AsRefStr, EnumString, Clone)]
pub enum ContactInfoType {
    Phone(String),
    WhatsApp(String),
    Email(String),
}

impl ContactInfoType {
    fn get_by_id(conn: &Connection, id: u64) -> Option<ContactInfoType> {
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

#[derive(Debug, Clone)]
pub struct Note {
    id: u64,
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
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
        let person = crate::Person::get_by_name(&conn, &person);
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

    pub fn update(&mut self, date: Option<String>, content: Option<String>) -> &Self {
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

        self
    }

    pub fn parse_from_editor(content: &str) -> Result<(String, Vec<String>), crate::ParseError> {
        let mut error = false;
        let mut note_contents: String = String::new();
        let mut people: Vec<String> = Vec::new();

        let content_prefix = "Content: ";
        let people_prefix = "People: ";

        content.lines().for_each(|line| match line {
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
            return Err(crate::ParseError::FormatError);
        }

        Ok((note_contents, people))
    }
}

impl crate::db::db_interface::DbOperations for Note {
    fn add(&self, conn: &Connection) -> Result<&Note, crate::db::db_interface::DbOperationsError> {
        let date_str = self.date.to_string();

        match conn.execute(
            "INSERT INTO 
                notes (date, content, deleted)
                VALUES (?1, ?2, FALSE)
            ",
            params![date_str, self.content],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        let id = &conn.last_insert_rowid();

        for person in &self.people {
            match conn.execute(
                "INSERT INTO people_notes (
                    person_id, 
                    note_id,
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

    fn remove(&self, conn: &crate::Connection) -> Result<&Self, db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE 
                    notes 
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

    fn save(&self, conn: &Connection) -> Result<&Note, crate::db::db_interface::DbOperationsError> {
        match conn.execute(
            "UPDATE
                notes
            SET
                date = ?1,
                content = ?2
            WHERE
                id = ?3",
            params![self.date.to_string(), self.content, self.id],
        ) {
            Ok(updated) => {
                println!("[DEBUG] {} rows were updated", updated);
            }
            Err(_) => return Err(crate::db::db_interface::DbOperationsError::GenericError),
        }

        Ok(self)
    }

    fn get_by_id(conn: &crate::Connection, id: u64) -> Option<Entities> {
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
                    recurring: crate::RecurringType::get_by_id(&conn, row.get(4).unwrap()).unwrap(),
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
