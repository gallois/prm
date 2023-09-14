pub mod activity;
pub mod event;
pub mod note;
pub mod person;
pub mod reminder;

use crate::db_interface::DbOperationsError;
use crate::entities::activity::Activity;
use crate::entities::note::Note;
use crate::entities::person::Person;
use crate::entities::reminder::Reminder;

#[derive(Debug)]
pub enum Entities {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
    Note(Note),
}

pub trait Entity {
    fn get_id(&self) -> u64;
}
