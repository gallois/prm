pub mod db;
pub mod editor;
pub mod entities;
pub mod helpers;

pub use crate::db::{db_helpers, db_interface};

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

pub static NOTE_TEMPLATE: &str = "Date: {date}
Content: {content}
People: {people}
";
