use chrono::NaiveDate;
use edit;

use prm::db_interface::DbOperations;
use prm::entities::activity::{Activity, ActivityType};
use prm::entities::note::{Note, NOTE_TEMPLATE};
use prm::entities::person::{ContactInfo, ContactInfoType, Person, PERSON_TEMPLATE};
use prm::entities::reminder::{RecurringType, Reminder, REMINDER_TEMPLATE};
use rusqlite::Connection;

extern crate strfmt;
use std::collections::HashMap;
use strfmt::strfmt;

use snafu::prelude::*;

// TODO Add more descriptive error messages
#[derive(Debug, Snafu)]
pub enum ParseError {
    #[snafu(display("Invalid birthday: {}", birthday))]
    BirthdayParseError { birthday: String },
    #[snafu(display("Invalid contact info: {}", contact_info))]
    ContactInfoParseError { contact_info: String },
    #[snafu(display("Invalid activity type: {}", activity_type))]
    ActivityTypeParseError { activity_type: String },
    #[snafu(display("Invalid date: {}", date))]
    DateParseError { date: String },
}

pub fn person(
    conn: &Connection,
    name: Option<String>,
    birthday: Option<String>,
    contact_info: Option<Vec<String>>,
) -> Result<Person, ParseError> {
    let mut name_str: String = String::new();
    let mut birthday_str: Option<String> = None;
    let mut contact_info_vec: Vec<String> = vec![];
    let mut editor = false;
    if name == None {
        editor = true;

        let mut vars = HashMap::new();
        vars.insert(
            "name".to_string(),
            prm::helpers::unwrap_arg_or_empty_string(name.clone()),
        );
        vars.insert(
            "birthday".to_string(),
            prm::helpers::unwrap_arg_or_empty_string(birthday.clone()),
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
        match prm::helpers::parse_from_str_ymd(&birthday_str) {
            Ok(date) => birthday_obj = Some(date),
            Err(_) => match prm::helpers::parse_from_str_md(&birthday_str) {
                Ok(date) => birthday_obj = Some(date),
                Err(_) => {
                    return BirthdayParseSnafu {
                        birthday: String::from(birthday_str),
                    }
                    .fail()
                }
            },
        }
    }

    let mut contact_info_splits: Vec<Vec<String>> = vec![];
    let mut contact_info_types: Vec<ContactInfoType> = vec![];

    match contact_info {
        Some(mut contact_info_vec) => {
            if !editor {
                ContactInfo::populate_splits(&mut contact_info_splits, &mut contact_info_vec);
            }
        }
        None => {
            if editor {
                ContactInfo::populate_splits(&mut contact_info_splits, &mut contact_info_vec);
            }
        }
    }

    let mut invalid_contact_info = vec![];
    if contact_info_splits.len() > 0 {
        contact_info_splits
            .into_iter()
            .for_each(|contact_info_split| match contact_info_split[0].as_str() {
                "phone" => {
                    contact_info_types.push(ContactInfoType::Phone(contact_info_split[1].clone()))
                }
                "whatsapp" => contact_info_types
                    .push(ContactInfoType::WhatsApp(contact_info_split[1].clone())),
                "email" => {
                    contact_info_types.push(ContactInfoType::Email(contact_info_split[1].clone()))
                }
                _ => {
                    invalid_contact_info.push(
                        vec![contact_info_split[0].clone(), contact_info_split[1].clone()]
                            .join(":"),
                    );
                }
            });
    }
    if !invalid_contact_info.is_empty() {
        return ContactInfoParseSnafu {
            contact_info: String::from(invalid_contact_info.join(",")),
        }
        .fail();
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
        // TODO better error handling
        Err(_) => panic!("Error while adding {}", person),
    };
    Ok(person)
}

pub fn activity(
    conn: &Connection,
    name: Option<String>,
    activity_type: Option<String>,
    date: Option<String>,
    content: Option<String>,
    people: Vec<String>,
) -> Result<Activity, ParseError> {
    let activity_vars: prm::helpers::ActivityVars;
    let mut vars = HashMap::new();
    vars.insert(
        "name".to_string(),
        prm::helpers::unwrap_arg_or_empty_string(name.clone()),
    );
    vars.insert(
        "date".to_string(),
        prm::helpers::unwrap_arg_or_empty_string(date.clone()),
    );
    vars.insert(
        "activity_type".to_string(),
        prm::helpers::unwrap_arg_or_empty_string(activity_type.clone()),
    );
    vars.insert(
        "content".to_string(),
        prm::helpers::unwrap_arg_or_empty_string(content.clone()),
    );
    vars.insert(
        "people".to_string(),
        if people.is_empty() {
            "".to_string()
        } else {
            people.clone().join(",")
        },
    );
    if name == None {
        activity_vars = prm::editor::populate_activity_vars(vars);
    } else {
        if [activity_type.clone(), date.clone(), content.clone()]
            .iter()
            .any(Option::is_none)
        {
            activity_vars = prm::editor::populate_activity_vars(vars);
        } else {
            activity_vars = prm::helpers::ActivityVars {
                name: name.unwrap(),
                date: date.unwrap(),
                activity_type: activity_type.unwrap(),
                content: content.unwrap(),
                people: people,
            };
        }
    }

    let activity_type = match activity_vars.activity_type.as_str() {
        "phone" => ActivityType::Phone,
        "in_person" => ActivityType::InPerson,
        "online" => ActivityType::Online,
        _ => {
            return ActivityTypeParseSnafu {
                activity_type: activity_vars.activity_type.clone(),
            }
            .fail()
        }
    };

    let date_obj = match prm::helpers::parse_from_str_ymd(activity_vars.date.as_str()) {
        Ok(date) => date,
        Err(_) => {
            return DateParseSnafu {
                date: String::from(activity_vars.date.clone()),
            }
            .fail()
        }
    };

    let people = Person::get_by_names(&conn, activity_vars.people);

    let activity = Activity::new(
        0,
        activity_vars.name,
        activity_type,
        date_obj,
        activity_vars.content,
        people,
    );
    match activity.add(&conn) {
        Ok(_) => println!("{:#?} added successfully", activity),
        // TODO better error handling
        Err(_) => panic!("Error while adding {:#?}", activity),
    };
    Ok(activity)
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
            prm::helpers::unwrap_arg_or_empty_string(name.clone()),
        );
        vars.insert(
            "date".to_string(),
            prm::helpers::unwrap_arg_or_empty_string(date.clone()),
        );
        vars.insert(
            "recurring_type".to_string(),
            prm::helpers::unwrap_arg_or_empty_string(recurring.clone()),
        );
        vars.insert(
            "description".to_string(),
            prm::helpers::unwrap_arg_or_empty_string(description.clone()),
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

    let date_obj = match prm::helpers::parse_from_str_ymd(date_string.as_str()) {
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
    let mut date_string: String = String::new();
    let mut content_string: String = String::new();
    let mut people_vec: Vec<Person> = Vec::new();

    if content == None {
        let mut vars = HashMap::new();
        vars.insert(
            "content".to_string(),
            prm::helpers::unwrap_arg_or_empty_string(content.clone()),
        );
        vars.insert("people".to_string(), people.clone().join(","));

        let edited = edit::edit(strfmt(NOTE_TEMPLATE, &vars).unwrap()).unwrap();
        let (d, c, p) = match Note::parse_from_editor(edited.as_str()) {
            Ok((date, content, people)) => (date, content, people),
            Err(_) => panic!("Error parsing note"),
        };
        date_string = d;
        content_string = c;
        people_vec = Person::get_by_names(&conn, p);
    }

    let date = match prm::helpers::parse_from_str_ymd(date_string.as_str()) {
        Ok(date) => date,
        Err(error) => panic!("Error parsing date: {}", error),
    };

    let note = Note::new(0, date, content_string, people_vec);
    println!("Note: {:#?}", note);
    match note.add(&conn) {
        Ok(_) => println!("{:#?} added successfully", note),
        Err(_) => panic!("Error while adding {:#?}", note),
    };
}
