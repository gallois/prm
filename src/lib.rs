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
    // TODO create a macro for generating all these `new` functions
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
    people: Vec<Person>,
}

impl Activity {
    pub fn new(
        name: String,
        activity_type: ActivityType,
        date: NaiveDate,
        content: String,
        people: Vec<Person>,
    ) -> Activity {
        Activity {
            name,
            activity_type,
            date,
            content,
            people,
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
pub struct Reminder {
    name: String,
    date: NaiveDate,
    recurring: Option<RecurringType>,
    people: Vec<Person>,
    description: Option<String>,
}

impl Reminder {
    pub fn new(
        name: String,
        date: NaiveDate,
        recurring: Option<RecurringType>,
        people: Vec<Person>,
        description: Option<String>,
    ) -> Reminder {
        Reminder {
            name,
            date,
            recurring,
            people,
            description,
        }
    }
}

#[derive(Debug)]
pub enum RecurringType {
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
pub struct Notes {
    date: NaiveDate,
    content: String,
    people: Vec<Person>,
}

impl Notes {
    pub fn new(date: NaiveDate, content: String, people: Vec<Person>) -> Notes {
        Notes {
            date,
            content,
            people,
        }
    }
}

enum Entity {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
    Notes(Notes),
}
