use chrono::prelude::*;

#[derive(Debug)]
pub struct Person {
    name: String,
    birthday: NaiveDate,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
}

impl Person {
    pub fn new(name: String, birthday: NaiveDate, contact_info: Vec<ContactInfo>) -> Person {
        Person {
            name,
            birthday,
            contact_info,
            activities: vec![],
            reminders: vec![],
        }
    }
}

#[derive(Debug)]
struct Activity {
    activity_type: ActivityType,
    name: String,
    date: NaiveDate,
    content: String,
}

#[derive(Debug)]
enum ActivityType {
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
}
