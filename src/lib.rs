use chrono::prelude::*;

struct Person {
    name: String,
    birthday: DateTime<Utc>,
    contact_info: Vec<ContactInfo>,
    activities: Vec<Activity>,
    reminders: Vec<Reminder>,
}

struct Activity {
    activity_type: ActivityType,
    name: String,
    date: DateTime<Utc>,
    content: String,
}

enum ActivityType {
    Phone,
    InPerson,
    Online,
}

struct Reminder {
    name: String,
    date: DateTime<Utc>,
    recurring: Option<RecurringType>,
    people: Vec<Person>,
}

enum RecurringType {
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Biannual,
    Yearly,
}

struct ContactInfo {
    contact_info_type: ContactInfoType,
}

enum ContactInfoType {
    Phone(String),
    Whatsapp(String),
    Email(String),
}

enum Entity {
    Person(Person),
    Activity(Activity),
    Reminder(Reminder),
}
