use crate::entities::Entity;
use std::any::type_name_of_val;
use std::{
    fmt::Display,
    io::{self, Write},
};

use snafu::Snafu;

use crate::entities::activity::ActivityType;
use crate::{ActivityTypeParseSnafu, CliError, ContactInfoParseSnafu};
use crate::entities::person::{ContactInfo, ContactInfoType};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub struct SelectionError {
    pub message: String,
}

pub struct ActivityVars {
    pub name: String,
    pub date: String,
    pub activity_type: String,
    pub content: String,
    pub people: Vec<String>,
}

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

pub fn handle_id_selection<T>(entity_vec: Vec<T>) -> Result<Vec<T>, SelectionError>
where
    T: Clone + Display + Entity,
{
    let entity_name = match type_name_of_val(&entity_vec[0]).split("::").last() {
        Some(entity_name) => entity_name.to_lowercase(),
        None => {
            return Err(SelectionError {
                message: format!("Invalid entity name: {}", type_name_of_val(&entity_vec[0])),
            })
        }
    };
    println!("Multiple {}s found", entity_name);
    for e in entity_vec.clone() {
        println!("[{}]\n{}", e.get_id(), e);
    }
    print!(
        "Which of the {} do you want to remove (0 to cancel)? ",
        entity_name
    );
    io::stdout().flush().unwrap();
    let mut n = String::new();
    io::stdin().read_line(&mut n).unwrap();
    let n = match n.trim().parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            return Err(SelectionError { message: String::from("Invalid input"),
            })
        }
    };
    if n == 0 {
        return Err(SelectionError {
            message: String::from("Aborted"),
        });
    }
    for e in entity_vec.clone() {
        if e.get_id() == n as u64 {
            return Ok(vec![e]);
        }
    }
    Err(SelectionError {
        message: String::from("Unknown error"),
    })
}

pub fn get_activity_type(activity_type: String) -> Result<ActivityType, CliError> {
    let activity_type = match activity_type.as_str() {
        "phone" => ActivityType::Phone,
        "in_person" => ActivityType::InPerson,
        "online" => ActivityType::Online,
        _ => {
            return ActivityTypeParseSnafu {
                activity_type: activity_type.to_string(),
            }.fail()
        }
    };
    Ok(activity_type)
}

pub fn get_contact_info(id: u64, splits: Vec<Vec<String>>) -> Result<Vec<ContactInfo>, CliError> {
    let mut contact_info_vec: Vec<ContactInfo> = Vec::new();
    let mut invalid_contact_info = vec![];
    let mut contact_info_type: Option<ContactInfoType>;

    for split in splits.iter() {
        match split[0].as_str() {
            "phone" => {
                contact_info_type =
                    Some(ContactInfoType::Phone(split[1].clone()))
            }
            "whatsapp" => {
                contact_info_type =
                    Some(ContactInfoType::WhatsApp(split[1].clone()))
            }
            "email" => {
                contact_info_type =
                    Some(ContactInfoType::Email(split[1].clone()))
            }
            _ => {
                invalid_contact_info.push(
                    [split[0].clone(), split[1].clone()]
                        .join(":"),
                );
                return ContactInfoParseSnafu {
                    contact_info: invalid_contact_info.join(","),
                }
                    .fail();
            }
        }

        if let Some(contact_info_type) = contact_info_type {
            contact_info_vec.push(ContactInfo::new(0, id, contact_info_type));
        }
    }
    Ok(contact_info_vec)
}