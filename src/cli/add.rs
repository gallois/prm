use chrono::NaiveDate;
use edit;

use prm::db_interface::DbOperations;
use prm::entities::activity::Activity;
use prm::entities::note::{Note, NOTE_TEMPLATE};
use prm::entities::person::{ContactInfo, Person, PERSON_TEMPLATE};
use prm::entities::reminder::{
    ParseReminderFromEditorData, RecurringType, Reminder, REMINDER_TEMPLATE,
};
use prm::{
    AddSnafu, BirthdayParseSnafu, CliError, DateParseSnafu, EditorParseSnafu, EntitySnafu,
    MissingFieldSnafu, NotFoundSnafu, RecurringTypeParseSnafu, TemplateSnafu,
};
use rusqlite::Connection;

extern crate strfmt;
use prm::helpers::{
    get_activity_type, get_contact_info, parse_from_str_md, parse_from_str_ymd,
    unwrap_arg_or_empty_string, ActivityVars,
};
use std::collections::HashMap;
use strfmt::strfmt;

pub fn person(
    conn: &Connection,
    name: Option<String>,
    birthday: Option<String>,
    contact_info: Option<Vec<String>>,
) -> Result<Person, CliError> {
    let mut name_str: String = String::new();
    let mut birthday_str: Option<String> = None;
    let mut contact_info_vec: Vec<String> = vec![];
    let mut editor = false;
    let mut activity_ids: Vec<u64> = vec![];
    let mut reminder_ids: Vec<u64> = vec![];

    if name.is_none() {
        editor = true;

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), unwrap_arg_or_empty_string(name.clone()));
        vars.insert(
            "birthday".to_string(),
            unwrap_arg_or_empty_string(birthday.clone()),
        );
        vars.insert(
            "contact_info".to_string(),
            contact_info.clone().unwrap_or_default().join(","),
        );

        let person_str = match strfmt(PERSON_TEMPLATE, &vars) {
            Ok(person_str) => person_str,
            Err(_) => {
                return TemplateSnafu {
                    template: PERSON_TEMPLATE,
                    vars,
                }
                .fail()
            }
        };
        let edited = match edit::edit(person_str) {
            Ok(edited) => edited,
            Err(e) => {
                return EditorParseSnafu {
                    entity: "Person",
                    message: format!("Error editing person: {:#?}", e),
                }
                .fail()
            }
        };
        let (n, b, c, a, r) = match Person::parse_from_editor(edited.as_str()) {
            Ok(d) => (
                d.name,
                d.birthday,
                d.contact_info,
                d.activities,
                d.reminders,
            ),
            Err(e) => {
                return EditorParseSnafu {
                    entity: "Person",
                    message: format!("Error editing person: {:#?}", e),
                }
                .fail()
            }
        };
        name_str = n;
        birthday_str = b;
        contact_info_vec = c;
        activity_ids = a;
        reminder_ids = r;
    }

    if !editor {
        name_str = match name {
            Some(name) => name,
            None => {
                return MissingFieldSnafu {
                    entity: "Person",
                    field: "Name",
                }
                .fail()
            }
        };
    }
    let mut birthday_obj: Option<NaiveDate> = None;
    if !editor {
        if let Some(bday) = birthday {
            birthday_str = Some(bday);
        }
    }

    if let Some(birthday_str) = birthday_str {
        match parse_from_str_ymd(&birthday_str) {
            Ok(date) => birthday_obj = Some(date),
            Err(_) => match parse_from_str_md(&birthday_str) {
                Ok(date) => birthday_obj = Some(date),
                Err(_) => {
                    return BirthdayParseSnafu {
                        birthday: birthday_str,
                    }
                    .fail()
                }
            },
        }
    }

    let mut contact_info_splits: Vec<Vec<String>> = vec![];

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

    let contact_info = get_contact_info(0, contact_info_splits)?;

    let mut activities: Vec<Activity> = vec![];
    for id in activity_ids {
        match Activity::get_by_id(conn, id) {
            Ok(entity) => match entity {
                Some(prm::entities::Entities::Activity(activity)) => activities.push(activity),
                _ => {
                    return EntitySnafu {
                        entity: "Activity",
                        message: format!("Wrong entity type: {:#?}", entity),
                    }
                    .fail()
                }
            },
            Err(_) => {
                return NotFoundSnafu {
                    entity: "Activity",
                    id,
                }
                .fail()
            }
        }
    }

    let mut reminders: Vec<Reminder> = vec![];
    for i in reminder_ids {
        match Reminder::get_by_id(conn, i) {
            Ok(entity) => match entity {
                Some(prm::entities::Entities::Reminder(reminder)) => reminders.push(reminder),
                _ => {
                    return EntitySnafu {
                        entity: "Reminder",
                        message: format!("Wrong entity type: {:#?}", entity),
                    }
                    .fail()
                }
            },
            Err(_) => {
                return NotFoundSnafu {
                    entity: "Reminder",
                    id: i,
                }
                .fail()
            }
        }
    }

    assert!(!name_str.is_empty(), "Name cannot be empty");
    let person = Person::new(
        0,
        name_str,
        birthday_obj,
        contact_info,
        activities,
        reminders,
    );
    match person.add(conn) {
        Ok(_) => println!("{} added successfully", person),
        Err(_) => return AddSnafu { entity: "Person" }.fail(),
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
) -> Result<Activity, CliError> {
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), unwrap_arg_or_empty_string(name.clone()));
    vars.insert("date".to_string(), unwrap_arg_or_empty_string(date.clone()));
    vars.insert(
        "activity_type".to_string(),
        unwrap_arg_or_empty_string(activity_type.clone()),
    );
    vars.insert(
        "content".to_string(),
        unwrap_arg_or_empty_string(content.clone()),
    );
    vars.insert(
        "people".to_string(),
        if people.is_empty() {
            "".to_string()
        } else {
            people.clone().join(",")
        },
    );
    let activity_vars: ActivityVars = if name.is_none()
        || [activity_type.clone(), date.clone(), content.clone()]
            .iter()
            .any(Option::is_none)
    {
        match prm::editor::populate_activity_vars(vars) {
            Ok(activity_vars) => activity_vars,
            Err(err) => {
                return EditorParseSnafu {
                    entity: err.to_string(),
                    message: format!("Error editing activity: {:#?}", err),
                }
                .fail()
            }
        }
    } else {
        let entity = "Activity";
        let name = match name {
            Some(name) => name,
            None => {
                return MissingFieldSnafu {
                    entity: String::from(entity),
                    field: "Name",
                }
                .fail()
            }
        };
        let date = match date {
            Some(date) => date,
            None => {
                return MissingFieldSnafu {
                    entity: String::from(entity),
                    field: "Date",
                }
                .fail()
            }
        };
        let activity_type = match activity_type {
            Some(activity_type) => activity_type,
            None => {
                return MissingFieldSnafu {
                    entity: String::from(entity),
                    field: "Activity Type",
                }
                .fail()
            }
        };
        let content = match content {
            Some(content) => content,
            None => {
                return MissingFieldSnafu {
                    entity: String::from(entity),
                    field: "Content",
                }
                .fail()
            }
        };
        ActivityVars {
            name,
            date,
            activity_type,
            content,
            people,
        }
    };

    let activity_type = get_activity_type(activity_vars.activity_type.clone())?;

    let date_obj = match parse_from_str_ymd(activity_vars.date.as_str()) {
        Ok(date) => date,
        Err(_) => {
            return DateParseSnafu {
                date: activity_vars.date.clone(),
            }
            .fail()
        }
    };

    let people = match prm::db::db_helpers::people::get_by_names(conn, activity_vars.people) {
        Ok(people) => people,
        Err(e) => {
            return EntitySnafu {
                entity: String::from("Person"),
                message: format!("Error fetching person: {:#?}", e),
            }
            .fail()
        }
    };

    let activity = Activity::new(
        0,
        activity_vars.name,
        activity_type,
        date_obj,
        activity_vars.content,
        people,
    );
    match activity.add(conn) {
        Ok(_) => println!("{:#?} added successfully", activity),
        Err(_) => return AddSnafu { entity: "Activity" }.fail(),
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
) -> Result<Reminder, CliError> {
    let mut name_string: String = String::new();
    let mut date_string: String = String::new();
    let mut recurring_type_string: String = String::new();
    let mut description_string: String = String::new();

    let mut editor = false;
    let entity = String::from("Reminder");
    if name.is_none() {
        editor = true;

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), unwrap_arg_or_empty_string(name.clone()));
        vars.insert("date".to_string(), unwrap_arg_or_empty_string(date.clone()));
        vars.insert(
            "recurring_type".to_string(),
            unwrap_arg_or_empty_string(recurring.clone()),
        );
        vars.insert(
            "description".to_string(),
            unwrap_arg_or_empty_string(description.clone()),
        );
        vars.insert(
            "people".to_string(),
            if people.is_empty() {
                "".to_string()
            } else {
                people.clone().join(",")
            },
        );

        let reminder_str = match strfmt(REMINDER_TEMPLATE, &vars) {
            Ok(reminder_str) => reminder_str,
            Err(_) => {
                return TemplateSnafu {
                    template: REMINDER_TEMPLATE,
                    vars,
                }
                .fail()
            }
        };
        let edited = match edit::edit(reminder_str) {
            Ok(edited) => edited,
            Err(e) => {
                return EditorParseSnafu {
                    entity: "Reminder",
                    message: format!("Error editing reminder: {:#?}", e),
                }
                .fail()
            }
        };
        let (n, da, r, de, p) = match Reminder::parse_from_editor(edited.as_str()) {
            Ok(ParseReminderFromEditorData {
                name,
                date,
                recurring_type,
                description,
                people,
            }) => (name, date, recurring_type, description, people),
            Err(e) => {
                return EditorParseSnafu {
                    entity: "Reminder",
                    message: format!("Error editing reminder: {:#?}", e),
                }
                .fail()
            }
        };
        name_string = n;
        date_string = match da {
            Some(da) => da,
            None => {
                return MissingFieldSnafu {
                    entity,
                    field: "Date",
                }
                .fail()
            }
        };
        recurring_type_string = match r {
            Some(r) => r,
            None => {
                return MissingFieldSnafu {
                    entity,
                    field: "Recurring Type",
                }
                .fail()
            }
        };
        description_string = match de {
            Some(de) => de,
            None => {
                return MissingFieldSnafu {
                    entity,
                    field: "Description",
                }
                .fail()
            }
        };
        people = p;
    }

    if !editor {
        name_string = match name {
            Some(name) => name,
            None => {
                return MissingFieldSnafu {
                    entity,
                    field: "Name",
                }
                .fail()
            }
        };
        date_string = match date {
            Some(date) => date,
            None => {
                return MissingFieldSnafu {
                    entity,
                    field: "Date",
                }
                .fail()
            }
        };
        recurring_type_string = match recurring {
            Some(recurring) => recurring,
            None => {
                return MissingFieldSnafu {
                    entity,
                    field: "Recurring Type",
                }
                .fail()
            }
        };
        description_string = description.unwrap_or("".to_string());
    }

    let recurring_type_str = recurring_type_string;
    let recurring_type = match recurring_type_str.as_str() {
        "daily" => RecurringType::Daily,
        "weekly" => RecurringType::Weekly,
        "fortnightly" => RecurringType::Fortnightly,
        "monthly" => RecurringType::Monthly,
        "quarterly" => RecurringType::Quarterly,
        "biannual" => RecurringType::Biannual,
        "yearly" => RecurringType::Yearly,
        "onetime" => RecurringType::OneTime,
        _ => {
            return RecurringTypeParseSnafu {
                recurring_type: recurring_type_str.clone(),
            }
            .fail()
        }
    };

    let date_obj = match parse_from_str_ymd(date_string.as_str()) {
        Ok(date) => date,
        Err(_) => {
            return DateParseSnafu {
                date: date_string.clone(),
            }
            .fail()
        }
    };

    let people = match prm::db::db_helpers::people::get_by_names(conn, people) {
        Ok(people) => people,
        Err(e) => {
            return EntitySnafu {
                entity: String::from("people"),
                message: format!("Error fetching people: {:#?}", e),
            }
            .fail()
        }
    };

    let reminder = Reminder::new(
        0,
        name_string,
        date_obj,
        Some(description_string),
        recurring_type,
        people,
    );
    println!("Reminder: {:#?}", reminder);
    match reminder.add(conn) {
        Ok(_) => println!("{:#?} added successfully", reminder),
        Err(_) => return AddSnafu { entity: "Reminder" }.fail(),
    };
    Ok(reminder)
}

pub fn note(
    conn: &Connection,
    content: Option<String>,
    people: Vec<String>,
) -> Result<Note, CliError> {
    let mut date_string: String = String::new();
    let mut content_string: String = String::new();
    let mut people_vec: Vec<Person> = Vec::new();
    let entity = String::from("Note");

    if content.is_none() {
        let mut vars = HashMap::new();
        vars.insert(
            "content".to_string(),
            unwrap_arg_or_empty_string(content.clone()),
        );
        vars.insert("people".to_string(), people.clone().join(","));

        let note_str = match strfmt(NOTE_TEMPLATE, &vars) {
            Ok(note_str) => note_str,
            Err(_) => {
                return TemplateSnafu {
                    template: NOTE_TEMPLATE,
                    vars,
                }
                .fail()
            }
        };
        let edited = match edit::edit(note_str) {
            Ok(edited) => edited,
            Err(e) => {
                return EditorParseSnafu {
                    entity,
                    message: format!("Error editing entity: {:#?}", e),
                }
                .fail()
            }
        };
        let (d, c, p) = match Note::parse_from_editor(edited.as_str()) {
            Ok((date, content, people)) => (date, content, people),
            Err(e) => {
                return EditorParseSnafu {
                    entity,
                    message: format!("Error editing entity: {:#?}", e),
                }
                .fail()
            }
        };
        date_string = d;
        content_string = c;
        people_vec = match prm::db::db_helpers::people::get_by_names(conn, p) {
            Ok(people) => people,
            Err(e) => {
                return EntitySnafu {
                    entity: String::from("people"),
                    message: format!("Error fetching people: {:#?}", e),
                }
                .fail()
            }
        };
    }

    let date = match parse_from_str_ymd(date_string.as_str()) {
        Ok(date) => date,
        Err(_) => {
            return DateParseSnafu {
                date: date_string.clone(),
            }
            .fail()
        }
    };

    let note = Note::new(0, date, content_string, people_vec);
    println!("Note: {:#?}", note);
    match note.add(conn) {
        Ok(_) => println!("{:#?} added successfully", note),
        Err(_) => return AddSnafu { entity }.fail(),
    };
    Ok(note)
}
