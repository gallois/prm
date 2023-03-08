use chrono::prelude::*;

#[derive(Debug)]
pub struct Person {
    name: String,
    birthday: Option<NaiveDate>,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
    notes: Vec<Notes>,
}

impl Person {
    pub fn new(
        name: String,
        birthday: Option<NaiveDate>,
        contact_info: Vec<ContactInfo>,
    ) -> Person {
        Person {
            name,
            birthday,
            contact_info,
            activities: vec![],
            reminders: vec![],
            notes: vec![],
        }
    }
}

#[derive(Debug)]
pub struct Activity {
    name: String,
    activity_type: ActivityType,
    date: NaiveDate,
    content: String,
}

impl Activity {
    pub fn new(
        name: String,
        activity_type: ActivityType,
        date: NaiveDate,
        content: String,
    ) -> Activity {
        Activity {
            name,
            activity_type,
            date,
            content,
        }
    }
}

#[derive(Debug)]
pub enum ActivityType {
    Phone,
    InPerson,
    Online,
}

#[derive(Debug)]
struct Reminder {
    name: String,
    date: NaiveDate,
    recurring: Option<RecurringType>,
    people: Vec<Person>,
}

#[derive(Debug)]
enum RecurringType {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Biannual,
    Yearly,
}

#[derive(Debug)]
pub struct ContactInfo {
    pub contact_info_type: ContactInfoType,
}

#[derive(Debug)]
pub enum ContactInfoType {
    Phone(String),
    Whatsapp(String),
    Email(String),
}

#[derive(Debug)]
enum Entity {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
    Notes(Notes),
}

#[derive(Debug)]
struct Notes {
    date: NaiveDate,
    content: String,
}
