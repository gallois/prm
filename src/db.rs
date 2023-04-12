pub mod db_interface {
    #[derive(Debug)]
    pub enum DbOperationsError {
        DuplicateEntry,
        GenericError,
    }

    pub trait DbOperations {
        fn add(&self, conn: &crate::Connection) -> Result<&Self, DbOperationsError>;
        fn remove(&self, conn: &crate::Connection) -> Result<&Self, DbOperationsError>;
        fn save(&self, conn: &crate::Connection) -> Result<&Self, DbOperationsError>;
        fn get_by_id(conn: &crate::Connection, id: u64) -> Option<crate::Entities>;
        // TODO get_all
    }
}

pub mod db_helpers {
    use crate::ContactInfoType;

    pub fn get_notes_by_person(conn: &crate::Connection, person_id: u64) -> Vec<crate::Note> {
        let mut stmt = conn
            .prepare(
                "SELECT
            *
        FROM
            people_notes
        WHERE
            person_id = ?
            AND deleted = FALSE
        ",
            )
            .unwrap();

        let mut rows = stmt.query(crate::params![person_id]).unwrap();
        let mut note_ids: Vec<u64> = vec![];
        while let Some(row) = rows.next().unwrap() {
            note_ids.push(row.get(0).unwrap());
        }

        if note_ids.is_empty() {
            return vec![];
        }

        let vars = crate::helpers::repeat_vars(note_ids.len());
        let sql = format!(
            "SELECT * from notes WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");

        let rows = stmt
            .query_map(crate::params_from_iter(note_ids.iter()), |row| {
                Ok(crate::Note::new(
                    row.get(0).unwrap(),
                    crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(1).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    row.get(2).unwrap(),
                    vec![],
                ))
            })
            .unwrap();

        let mut notes = vec![];
        for note in rows {
            notes.push(note.unwrap());
        }

        notes
    }

    pub fn get_reminders_by_person(
        conn: &crate::Connection,
        person_id: u64,
    ) -> Vec<crate::Reminder> {
        let mut stmt = conn
            .prepare(
                "SELECT
            reminder_id
        FROM
            people_reminders
        WHERE
            person_id = ?
            AND deleted = FALSE
        ",
            )
            .unwrap();

        let mut rows = stmt.query(crate::params![person_id]).unwrap();
        let mut reminder_ids: Vec<u64> = vec![];
        while let Some(row) = rows.next().unwrap() {
            reminder_ids.push(row.get(0).unwrap());
        }

        if reminder_ids.is_empty() {
            return vec![];
        }

        let vars = crate::helpers::repeat_vars(reminder_ids.len());
        let sql = format!(
            "SELECT * FROM reminders WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");

        let rows = stmt
            .query_map(crate::params_from_iter(reminder_ids.iter()), |row| {
                Ok(crate::Reminder::new(
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    row.get(3).unwrap(),
                    crate::RecurringType::get_by_id(&conn, row.get(4).unwrap()),
                    vec![],
                ))
            })
            .unwrap();

        let mut reminders = vec![];
        for reminder in rows {
            reminders.push(reminder.unwrap());
        }

        reminders
    }

    pub fn get_contact_info_by_person(
        conn: &crate::Connection,
        person_id: u64,
    ) -> Vec<crate::ContactInfo> {
        let mut stmt = conn
            .prepare(
                "SELECT 
                * 
            FROM
                contact_info
            WHERE
                person_id = ?
                AND deleted = FALSE
            ",
            )
            .unwrap();

        let mut contact_info_vec: Vec<crate::ContactInfo> = vec![];
        let rows = stmt
            .query_map(crate::params![person_id], |row| {
                let contact_info_type =
                    crate::ContactInfoType::get_by_id(&conn, row.get(2).unwrap()).unwrap();
                let contact_info_details: String = row.get(3).unwrap();
                let contact_info = match contact_info_type {
                    ContactInfoType::Phone(_) => ContactInfoType::Phone(contact_info_details),
                    ContactInfoType::WhatsApp(_) => ContactInfoType::WhatsApp(contact_info_details),
                    ContactInfoType::Email(_) => ContactInfoType::Email(contact_info_details),
                };

                Ok(crate::ContactInfo::new(
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    contact_info,
                ))
            })
            .unwrap();

        for contact_info in rows {
            contact_info_vec.push(contact_info.unwrap());
        }

        contact_info_vec
    }

    pub fn get_activities_by_person(
        conn: &crate::Connection,
        person_id: u64,
    ) -> Vec<crate::Activity> {
        let mut stmt = conn
            .prepare(
                "SELECT 
                activity_id 
            FROM
                people_activities
            WHERE
                person_id = ?
                AND deleted = FALSE
            ",
            )
            .unwrap();

        let mut rows = stmt.query(crate::params![person_id]).unwrap();
        let mut activity_ids: Vec<u64> = vec![];
        while let Some(row) = rows.next().unwrap() {
            activity_ids.push(row.get(0).unwrap());
        }

        if activity_ids.is_empty() {
            return vec![];
        }

        let vars = crate::helpers::repeat_vars(activity_ids.len());
        let sql = format!(
            "SELECT * FROM activities WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");

        let rows = stmt
            .query_map(crate::params_from_iter(activity_ids.iter()), |row| {
                Ok(crate::Activity::new(
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    crate::ActivityType::get_by_id(&conn, row.get(2).unwrap()).unwrap(),
                    crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                    row.get(4).unwrap(),
                    vec![],
                ))
            })
            .unwrap();

        let mut activities = vec![];
        for activity in rows {
            activities.push(activity.unwrap());
        }

        activities
    }

    // TODO remove duplication with similar functions
    pub fn get_people_by_reminder(
        conn: &crate::Connection,
        reminder_id: u64,
    ) -> Vec<crate::Person> {
        let mut stmt = conn
            .prepare(
                "SELECT
                        person_id
                    FROM
                        people_reminders
                    WHERE
                        reminder_id = ?
                        AND deleted = FALSE
            ",
            )
            .expect("Invalid SQL statement");

        let mut rows = stmt.query(crate::params![reminder_id]).unwrap();
        let mut people_ids: Vec<u64> = vec![];
        while let Some(row) = rows.next().unwrap() {
            people_ids.push(row.get(0).unwrap());
        }

        if people_ids.is_empty() {
            return vec![];
        }

        let vars = crate::helpers::repeat_vars(people_ids.len());
        let sql = format!(
            "SELECT * FROM people WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");

        let rows = stmt
            .query_map(crate::params_from_iter(people_ids.iter()), |row| {
                let person_id = row.get(0).unwrap();
                Ok(crate::Person {
                    id: person_id,
                    name: row.get(1).unwrap(),
                    birthday: Some(
                        crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    contact_info: crate::db::db_helpers::get_contact_info_by_person(
                        &conn, person_id,
                    ),
                    activities: crate::db::db_helpers::get_activities_by_person(&conn, person_id),
                    reminders: crate::db::db_helpers::get_reminders_by_person(&conn, person_id),
                    notes: crate::db::db_helpers::get_notes_by_person(&conn, person_id),
                })
            })
            .unwrap();

        let mut activities = vec![];
        for activity in rows {
            activities.push(activity.unwrap());
        }

        activities
    }

    // TODO remove duplication with similar functions
    pub fn get_people_by_activity(
        conn: &crate::Connection,
        activity_id: u64,
    ) -> Vec<crate::Person> {
        let mut stmt = conn
            .prepare(
                "SELECT
                        person_id
                    FROM
                        people_activities
                    WHERE
                        activity_id = ?
                        AND deleted = FALSE
            ",
            )
            .expect("Invalid SQL statement");

        let mut rows = stmt.query(crate::params![activity_id]).unwrap();
        let mut people_ids: Vec<u64> = vec![];
        while let Some(row) = rows.next().unwrap() {
            people_ids.push(row.get(0).unwrap());
        }

        if people_ids.is_empty() {
            return vec![];
        }

        let vars = crate::helpers::repeat_vars(people_ids.len());
        let sql = format!(
            "SELECT * FROM people WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");

        let rows = stmt
            .query_map(crate::params_from_iter(people_ids.iter()), |row| {
                let person_id = row.get(0).unwrap();
                Ok(crate::Person {
                    id: person_id,
                    name: row.get(1).unwrap(),
                    birthday: Some(
                        crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    contact_info: crate::db::db_helpers::get_contact_info_by_person(
                        &conn, person_id,
                    ),
                    activities: crate::db::db_helpers::get_activities_by_person(&conn, person_id),
                    reminders: crate::db::db_helpers::get_reminders_by_person(&conn, person_id),
                    notes: crate::db::db_helpers::get_notes_by_person(&conn, person_id),
                })
            })
            .unwrap();

        let mut people = vec![];
        for person in rows {
            people.push(person.unwrap());
        }

        people
    }

    pub fn get_people_by_note(conn: &crate::Connection, note_id: u64) -> Vec<crate::Person> {
        let mut stmt = conn
            .prepare(
                "SELECT
                        person_id
                    FROM
                        people_notes
                    WHERE
                        note_id = ?
                        AND deleted = FALSE
                    ",
            )
            .expect("Invalid SQL statement");

        let mut rows = stmt.query(crate::params![note_id]).unwrap();
        let mut people_ids: Vec<u64> = vec![];
        while let Some(row) = rows.next().unwrap() {
            people_ids.push(row.get(0).unwrap());
        }

        if people_ids.is_empty() {
            return vec![];
        }

        let vars = crate::helpers::repeat_vars(people_ids.len());
        let sql = format!(
            "SELECT * FROM people WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");

        let rows = stmt
            .query_map(crate::params_from_iter(people_ids.iter()), |row| {
                let person_id = row.get(0).unwrap();
                Ok(crate::Person {
                    id: person_id,
                    name: row.get(1).unwrap(),
                    birthday: Some(
                        crate::helpers::parse_from_str_ymd(
                            String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    contact_info: crate::db::db_helpers::get_contact_info_by_person(
                        &conn, person_id,
                    ),
                    activities: crate::db::db_helpers::get_activities_by_person(&conn, person_id),
                    reminders: crate::db::db_helpers::get_reminders_by_person(&conn, person_id),
                    notes: crate::db::db_helpers::get_notes_by_person(&conn, person_id),
                })
            })
            .unwrap();

        let mut notes = vec![];
        for note in rows {
            notes.push(note.unwrap());
        }

        notes
    }
    pub fn init_db(
        conn: &crate::Connection,
    ) -> Result<(), crate::db::db_interface::DbOperationsError> {
        let sql_create_statements = vec![
            "CREATE TABLE people (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            birthday TEXT,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE activities (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            type INTEGER NOT NULL,
            date TEXT NOT NULL,
            content TEXT,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE reminders (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            date TEXT NOT NULL,
            description TEXT,
            recurring INTEGER NOT NULL,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE notes (
            id INTEGER PRIMARY KEY, 
            date TEXT NOT NULL,
            content TEXT NOT NULL,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE contact_info (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            contact_info_type_id INTEGER NOT NULL,
            contact_info_details TEXT,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE contact_info_types (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE people_activities (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            activity_id INTEGER NOT NULL,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE people_reminders (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            reminder_id INTEGER NOT NULL,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE people_notes (
            id INTEGER PRIMARY KEY,
            person_id INTEGER NOT NULL,
            note_id INTEGER NOT NULL,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE activity_types (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL,
            deleted INTEGER NOT NULL
        );",
            "CREATE TABLE recurring_types (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL,
            deleted INTEGER NOT NULL
        );",
        ];
        for query in sql_create_statements {
            match conn.execute(query, ()) {
                // Improve message
                Ok(_) => println!("Database table created"),
                Err(error) => {
                    println!("Error creating database tables: {}", error);
                    return Err(crate::db::db_interface::DbOperationsError::GenericError);
                }
            }
        }
        let sql_populate_statements = vec![
            "INSERT INTO contact_info_types (type, deleted)
         VALUES 
            ('Phone', FALSE),
            ('WhatsApp', FALSE),
            ('Email', FALSE)
        ",
            "INSERT INTO activity_types (type, deleted)
         VALUES 
            ('Phone', FALSE),
            ('InPerson', FALSE),
            ('Online', FALSE)
        ",
            "INSERT INTO recurring_types (type, deleted)
         VALUES
            ('Daily', FALSE),
            ('Weekly', FALSE),
            ('Fortnightly', FALSE),
            ('Monthly', FALSE),
            ('Quarterly', FALSE),
            ('Biannual', FALSE),
            ('Yearly', FALSE)
        ",
        ];
        for query in sql_populate_statements {
            match conn.execute(query, ()) {
                // Improve message
                Ok(_) => println!("Database table populated"),
                Err(error) => {
                    println!("Error populating database tables: {}", error);
                    return Err(crate::db::db_interface::DbOperationsError::GenericError);
                }
            }
        }
        Ok(())
    }
}
