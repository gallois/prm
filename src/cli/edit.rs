use prm::db::db_interface::DbOperations;
use prm::entities::activity::{Activity, ParseActivityFromEditorData, ACTIVITY_TEMPLATE};
use prm::entities::note::{Note, NOTE_TEMPLATE};
use prm::entities::person::{Person, PERSON_TEMPLATE};
use prm::entities::reminder::{ParseReminderFromEditorData, Reminder, REMINDER_TEMPLATE};
use prm::entities::Entities;
extern crate strfmt;
use rusqlite::Connection;
use std::collections::HashMap;
use strfmt::strfmt;

use prm::CliError;
use prm::{EditSnafu, EditorParseSnafu, EntitySnafu, NotFoundSnafu, TemplateSnafu};

pub fn person(
    conn: &Connection,
    id: u64,
    name: Option<String>,
    birthday: Option<String>,
    contact_info: Option<String>,
    activities: Option<Vec<u64>>,
) -> Result<Person, CliError> {
    let name_str: String;
    let birthday_str: Option<String>;
    let contact_info_str: Option<String>;
    let activity_ids: Vec<u64>;

    let person = Person::get_by_id(conn, id);

    match person {
        Ok(person) => match person {
            Some(entity) => {
                let mut person = match entity {
                    Entities::Person(person) => person,
                    _ => {
                        return EntitySnafu {
                            entity: "Person".to_string(),
                            message: format!("Wrong entity type: {:#?}", entity),
                        }
                        .fail()
                    }
                };
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

                let activities_field = prm::helpers::join_int_vector(
                    person
                        .activities
                        .iter()
                        .map(|x| x.id)
                        .collect::<Vec<u64>>()
                        .as_slice(),
                );

                let mut vars = HashMap::new();
                let name_placeholder: String;
                if name.is_some() {
                    name_placeholder = match name {
                        Some(name) => name,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "person".to_string(),
                                field: "name".to_string(),
                            })
                        }
                    };
                } else if !person.name.is_empty() {
                    name_placeholder = person.name.clone();
                } else {
                    name_placeholder = "".to_string();
                }
                let birthday_placeholder: String;
                if birthday.is_some() {
                    birthday_placeholder = match birthday {
                        Some(birthday) => birthday,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "person".to_string(),
                                field: "birthday".to_string(),
                            })
                        }
                    };
                } else if person.birthday.is_some() {
                    birthday_placeholder = match person.birthday {
                        Some(birthday) => birthday.to_string(),
                        None => {
                            return Err(CliError::MissingField {
                                entity: "person".to_string(),
                                field: "birthday".to_string(),
                            })
                        }
                    };
                } else {
                    birthday_placeholder = "".to_string();
                }
                let contact_info_placeholder: String;
                if contact_info.is_some() {
                    contact_info_placeholder = match contact_info {
                        Some(contact_info) => contact_info,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "person".to_string(),
                                field: "contact info".to_string(),
                            })
                        }
                    };
                } else if !contact_info_field.is_empty() {
                    contact_info_placeholder = contact_info_field;
                } else {
                    contact_info_placeholder = "".to_string();
                }
                let activities_placeholder: String;
                if activities.is_some() {
                    activities_placeholder = match activities {
                        Some(activities) => prm::helpers::join_int_vector(activities.as_slice()),
                        None => {
                            return Err(CliError::MissingField {
                                entity: "person".to_string(),
                                field: "activities".to_string(),
                            })
                        }
                    }
                } else if !activities_field.is_empty() {
                    activities_placeholder = activities_field;
                } else {
                    activities_placeholder = "".to_string();
                }
                vars.insert("name".to_string(), name_placeholder);
                vars.insert("birthday".to_string(), birthday_placeholder);
                vars.insert("contact_info".to_string(), contact_info_placeholder);
                vars.insert("activities".to_string(), activities_placeholder);

                let person_str = match strfmt(PERSON_TEMPLATE, &vars) {
                    Ok(person_str) => person_str,
                    Err(_) => {
                        return {
                            TemplateSnafu {
                                template: PERSON_TEMPLATE,
                                vars,
                            }
                        }
                        .fail()
                    }
                };
                let edited = match edit::edit(person_str) {
                    Ok(edited) => edited,
                    Err(e) => {
                        return {
                            EditorParseSnafu {
                                entity: "Person".to_string(),
                                message: format!("Error editing person: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                let (n, b, c, a) = match Person::parse_from_editor(edited.as_str()) {
                    Ok(d) => (d.name, d.birthday, d.contact_info, d.activities),
                    Err(e) => {
                        return {
                            EditorParseSnafu {
                                entity: "Person".to_string(),
                                message: format!("Error editing person: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                name_str = n;
                birthday_str = b;
                contact_info_str = Some(c.join(","));
                activity_ids = a;

                let activities = match Activity::get_by_ids(conn, activity_ids) {
                    Ok(activities) => activities,
                    Err(e) => {
                        return {
                            EntitySnafu {
                                entity: "Activity".to_string(),
                                message: format!("Error fetching activities: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };

                match person.update(name_str, birthday_str, contact_info_str, activities) {
                    Ok(_) => (),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Person".to_string(),
                                message: format!("Error editing person: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                match person.save(conn) {
                    Ok(person) => println!("Updated person: {}", person),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Person".to_string(),
                                message: format!("Error editing person: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                }
                Ok(person)
            }
            None => {
                println!("Could not find person id {}", id);
                NotFoundSnafu {
                    entity: "Person".to_string(),
                    id,
                }
                .fail()
            }
        },
        Err(e) => EntitySnafu {
            entity: "Person".to_string(),
            message: format!("Error fetching person: {:#?}", e),
        }
        .fail(),
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
    let activity = Activity::get_by_id(conn, id);

    let name_string: String;
    let date_string: String;
    let activity_type_string: String;
    let content_string: String;
    let people: Vec<String>;

    match activity {
        Ok(activity) => match activity {
            Some(entity) => {
                let mut activity = match entity {
                    Entities::Activity(activity) => activity,
                    _ => {
                        return {
                            EntitySnafu {
                                entity: "Activity".to_string(),
                                message: format!("Wrong entity type: {:#?}", entity),
                            }
                            .fail()
                        }
                    }
                };

                let mut vars = HashMap::new();
                let name_placeholder: String;
                if name.clone().is_some() {
                    name_placeholder = match name.clone() {
                        Some(name) => name,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "activity".to_string(),
                                field: "name".to_string(),
                            })
                        }
                    };
                } else if !activity.name.is_empty() {
                    name_placeholder = activity.name.clone();
                } else {
                    name_placeholder = "".to_string();
                }
                let date_placeholder: String;
                if date.clone().is_some() {
                    date_placeholder = match date.clone() {
                        Some(date) => date,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "activity".to_string(),
                                field: "date".to_string(),
                            })
                        }
                    };
                } else if !activity.date.to_string().is_empty() {
                    date_placeholder = activity.date.clone().to_string();
                } else {
                    date_placeholder = "".to_string();
                }
                let activity_type_placeholder: String;
                if activity_type.clone().is_some() {
                    activity_type_placeholder = match activity_type.clone() {
                        Some(activity_type) => activity_type,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "activity".to_string(),
                                field: "activity type".to_string(),
                            })
                        }
                    };
                } else if !activity.activity_type.as_ref().is_empty() {
                    activity_type_placeholder =
                        activity.activity_type.as_ref().to_string().to_lowercase();
                } else {
                    activity_type_placeholder = "".to_string();
                }
                let content_placeholder: String;
                if content.clone().is_some() {
                    content_placeholder = match content.clone() {
                        Some(content) => content,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "activity".to_string(),
                                field: "content".to_string(),
                            })
                        }
                    };
                } else if !activity.content.is_empty() {
                    content_placeholder = activity.content.clone();
                } else {
                    content_placeholder = "".to_string();
                }
                let people_placeholder: String = if !activity.people.is_empty() {
                    activity
                        .people
                        .clone()
                        .iter()
                        .map(|p| p.clone().name)
                        .collect::<Vec<String>>()
                        .join(",")
                        .to_string()
                } else {
                    "".to_string()
                };

                vars.insert("name".to_string(), name_placeholder);
                vars.insert("date".to_string(), date_placeholder);
                vars.insert("activity_type".to_string(), activity_type_placeholder);
                vars.insert("content".to_string(), content_placeholder);
                vars.insert("people".to_string(), people_placeholder);

                let activity_str = match strfmt(ACTIVITY_TEMPLATE, &vars) {
                    Ok(activity_str) => activity_str,
                    Err(_) => {
                        return {
                            TemplateSnafu {
                                template: ACTIVITY_TEMPLATE,
                                vars,
                            }
                            .fail()
                        }
                    }
                };
                let edited = match edit::edit(activity_str) {
                    Ok(edited) => edited,
                    Err(e) => {
                        return {
                            EditorParseSnafu {
                                entity: "Activity".to_string(),
                                message: format!("Error editing activity: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                let (n, d, t, c, p) = match Activity::parse_from_editor(edited.as_str()) {
                    Ok(ParseActivityFromEditorData {
                        name,
                        date,
                        activity_type,
                        content,
                        people,
                    }) => (name, date, activity_type, content, people),
                    Err(e) => {
                        return {
                            EditorParseSnafu {
                                entity: "Activity".to_string(),
                                message: format!("Error editing activity: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                name_string = n;
                date_string = match d {
                    Some(d) => d,
                    None => {
                        return Err(CliError::MissingField {
                            entity: "activity".to_string(),
                            field: "date".to_string(),
                        })
                    }
                };
                activity_type_string = match t {
                    Some(t) => t,
                    None => {
                        return Err(CliError::MissingField {
                            entity: "activity".to_string(),
                            field: "activity type".to_string(),
                        })
                    }
                };
                content_string = match c {
                    Some(c) => c,
                    None => {
                        return Err(CliError::MissingField {
                            entity: "activity".to_string(),
                            field: "content".to_string(),
                        })
                    }
                };
                people = p;

                match activity.update(
                    conn,
                    Some(name_string),
                    Some(activity_type_string),
                    Some(date_string),
                    Some(content_string),
                    people,
                ) {
                    Ok(_) => (),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Activity".to_string(),
                                message: format!("Error editing activity: {:#?}", e),
                            }
                        }
                        .fail()
                    }
                };
                match activity.save(conn) {
                    Ok(activity) => println!("Updated activity: {:#?}", activity),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Activity".to_string(),
                                message: format!("Error editing activity: {:#?}", e),
                            }
                        }
                        .fail()
                    }
                }
                Ok(activity)
            }
            None => NotFoundSnafu {
                entity: "Activity".to_string(),
                id,
            }
            .fail(),
        },
        Err(e) => EntitySnafu {
            entity: "Activity".to_string(),
            message: format!("Error fetching activity: {:#?}", e),
        }
        .fail(),
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
    let reminder = Reminder::get_by_id(conn, id);

    let name_string: String;
    let date_string: String;
    let recurring_type_string: String;
    let description_string: String;
    let people: Vec<String>;

    match reminder {
        Ok(reminder) => match reminder {
            Some(entity) => {
                let mut reminder = match entity {
                    Entities::Reminder(reminder) => reminder,
                    _ => {
                        return {
                            EntitySnafu {
                                entity: "Reminder".to_string(),
                                message: format!("Wrong entity type: {:#?}", entity),
                            }
                            .fail()
                        }
                    }
                };

                let mut vars = HashMap::new();
                let name_placeholder: String;
                if name.is_some() {
                    name_placeholder = match name {
                        Some(name) => name,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "reminder".to_string(),
                                field: "name".to_string(),
                            })
                        }
                    }
                } else if !reminder.name.is_empty() {
                    name_placeholder = reminder.name.clone();
                } else {
                    name_placeholder = "".to_string();
                }
                let date_placeholder: String;
                if date.is_some() {
                    date_placeholder = match date {
                        Some(date) => date,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "reminder".to_string(),
                                field: "date".to_string(),
                            })
                        }
                    }
                } else if !reminder.date.to_string().is_empty() {
                    date_placeholder = reminder.date.clone().to_string();
                } else {
                    date_placeholder = "".to_string();
                }
                let description_placeholder: String;
                let reminder_description = match reminder.description.as_ref() {
                    Some(description) => description,
                    None => "",
                };
                if description.is_some() {
                    description_placeholder = match description {
                        Some(description) => description,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "reminder".to_string(),
                                field: "description".to_string(),
                            })
                        }
                    }
                } else if !reminder_description.is_empty() {
                    description_placeholder = String::from(reminder_description);
                } else {
                    description_placeholder = "".to_string();
                }
                let recurring_placeholder: String;
                if recurring.is_some() {
                    recurring_placeholder = match recurring {
                        Some(recurring) => recurring,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "reminder".to_string(),
                                field: "recurring type".to_string(),
                            })
                        }
                    };
                } else if !reminder.recurring.as_ref().is_empty() {
                    recurring_placeholder = reminder.recurring.as_ref().to_string().to_lowercase();
                } else {
                    recurring_placeholder = "".to_string();
                }
                let people_placeholder: String = if !reminder.people.is_empty() {
                    reminder
                        .people
                        .clone()
                        .iter()
                        .map(|p| p.clone().name)
                        .collect::<Vec<String>>()
                        .join(",")
                } else {
                    "".to_string()
                };
                vars.insert("date".to_string(), date_placeholder);
                vars.insert("name".to_string(), name_placeholder);
                vars.insert("description".to_string(), description_placeholder);
                vars.insert("recurring_type".to_string(), recurring_placeholder);
                vars.insert("people".to_string(), people_placeholder);

                let reminder_str = match strfmt(REMINDER_TEMPLATE, &vars) {
                    Ok(s) => s,
                    Err(_) => {
                        return {
                            TemplateSnafu {
                                template: "Reminder".to_string(),
                                vars,
                            }
                            .fail()
                        }
                    }
                };
                let edited = match edit::edit(reminder_str) {
                    Ok(edited) => edited,
                    Err(e) => {
                        return {
                            EditorParseSnafu {
                                entity: "Reminder".to_string(),
                                message: format!("Error editing reminder: {:#?}", e),
                            }
                            .fail()
                        }
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
                        return {
                            EditorParseSnafu {
                                entity: "Reminder".to_string(),
                                message: format!("Error parsing reminder: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                name_string = n;
                date_string = match da {
                    Some(da) => da,
                    None => {
                        return Err(CliError::MissingField {
                            entity: "reminder".to_string(),
                            field: "date".to_string(),
                        })
                    }
                };
                recurring_type_string = match r {
                    Some(r) => r,
                    None => {
                        return Err(CliError::MissingField {
                            entity: "reminder".to_string(),
                            field: "recurring type".to_string(),
                        })
                    }
                };
                description_string = match de {
                    Some(de) => de,
                    None => {
                        return Err(CliError::MissingField {
                            entity: "reminder".to_string(),
                            field: "description".to_string(),
                        })
                    }
                };
                people = p;

                match reminder.update(
                    conn,
                    Some(name_string),
                    Some(date_string),
                    Some(description_string),
                    Some(recurring_type_string),
                    people,
                ) {
                    Ok(_) => (),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Reminder".to_string(),
                                message: format!("Error editing reminder: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                match reminder.save(conn) {
                    Ok(reminder) => println!("Updated reminder: {:#?}", reminder),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Reminder".to_string(),
                                message: format!("Error editing reminder: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                }
                Ok(reminder)
            }
            None => NotFoundSnafu {
                entity: "Reminder".to_string(),
                id,
            }
            .fail(),
        },
        Err(e) => EntitySnafu {
            entity: "Reminder".to_string(),
            message: format!("Error fetching reminder: {:#?}", e),
        }
        .fail(),
    }
}
pub fn note(
    conn: &Connection,
    id: u64,
    date: Option<String>,
    content: Option<String>,
) -> Result<Note, CliError> {
    let note = Note::get_by_id(conn, id);

    let date_string: String;
    let content_string: String;
    let people: Vec<String>;

    match note {
        Ok(note) => match note {
            Some(entity) => {
                let mut note = match entity {
                    Entities::Note(note) => note,
                    _ => {
                        return {
                            EntitySnafu {
                                entity: "Note".to_string(),
                                message: format!("Wrong entity type: {:#?}", entity),
                            }
                        }
                        .fail()
                    }
                };
                let mut vars = HashMap::new();
                let date_placeholder: String;
                let content_placeholder: String;
                let people_placeholder: String = if !note.people.is_empty() {
                    note.people
                        .iter()
                        .map(|p| p.clone().name)
                        .collect::<Vec<String>>()
                        .join(",")
                } else {
                    "".to_string()
                };

                if date.is_some() {
                    date_placeholder = match date {
                        Some(date) => date,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "note".to_string(),
                                field: "date".to_string(),
                            })
                        }
                    }
                } else if !note.date.to_string().is_empty() {
                    date_placeholder = note.date.clone().to_string();
                } else {
                    date_placeholder = "".to_string();
                }
                if content.is_some() {
                    content_placeholder = match content {
                        Some(content) => content,
                        None => {
                            return Err(CliError::MissingField {
                                entity: "note".to_string(),
                                field: "content".to_string(),
                            })
                        }
                    }
                } else if !note.content.is_empty() {
                    content_placeholder = note.content.clone();
                } else {
                    content_placeholder = "".to_string();
                }

                vars.insert("date".to_string(), date_placeholder);
                vars.insert("content".to_string(), content_placeholder);
                vars.insert("people".to_string(), people_placeholder);

                let note_str = match strfmt(NOTE_TEMPLATE, &vars) {
                    Ok(s) => s,
                    Err(_) => {
                        return {
                            TemplateSnafu {
                                template: NOTE_TEMPLATE,
                                vars,
                            }
                            .fail()
                        }
                    }
                };
                let edited = match edit::edit(note_str) {
                    Ok(edited) => edited,
                    Err(e) => {
                        return {
                            EditorParseSnafu {
                                entity: "Note".to_string(),
                                message: format!("Error editing note: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                let (d, c, p) = match Note::parse_from_editor(edited.as_str()) {
                    Ok((date, content, people)) => (date, content, people),
                    Err(e) => {
                        return {
                            EditorParseSnafu {
                                entity: "Note".to_string(),
                                message: format!("Error parsing note: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };

                date_string = d;
                content_string = c;
                people = p;

                match note.update(conn, Some(date_string), Some(content_string), people) {
                    Ok(_) => (),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Note".to_string(),
                                message: format!("Error editing note: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                match note.save(conn) {
                    Ok(note) => println!("Updated note: {:#?}", note),
                    Err(e) => {
                        return {
                            EditSnafu {
                                entity: "Note".to_string(),
                                message: format!("Error editing note: {:#?}", e),
                            }
                            .fail()
                        }
                    }
                };
                println!("Updated note: {:#?}", note);
                Ok(note)
            }
            None => NotFoundSnafu {
                entity: "Note".to_string(),
                id,
            }
            .fail(),
        },
        Err(e) => EntitySnafu {
            entity: "Note".to_string(),
            message: format!("Wrong entity type: {:#?}", e),
        }
        .fail(),
    }
}
