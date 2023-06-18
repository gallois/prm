use rusqlite::{params, params_from_iter, Connection};

pub mod db_interface {
    use crate::db::Connection;

    #[derive(Debug)]
    pub enum DbOperationsError {
        DuplicateEntry,
        GenericError,
        InvalidStatement,
        QueryError,
        RecordError,
    }

    pub trait DbOperations {
        fn add(&self, conn: &Connection) -> Result<&Self, DbOperationsError>;
        fn remove(&self, conn: &Connection) -> Result<&Self, DbOperationsError>;
        fn save(&self, conn: &Connection) -> Result<&Self, DbOperationsError>;
        fn get_by_id(
            conn: &Connection,
            id: u64,
        ) -> Result<Option<crate::entities::Entities>, DbOperationsError>;
        // TODO get_all
    }
}

pub mod db_helpers {
    use crate::db::{params, params_from_iter, Connection};
    use crate::db_interface::DbOperationsError;
    use crate::entities::person::ContactInfoType;

    pub fn get_notes_by_person(
        conn: &Connection,
        person_id: u64,
    ) -> Result<Vec<crate::entities::note::Note>, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "SELECT
            *
        FROM
            people_notes
        WHERE
            person_id = ?
            AND deleted = FALSE
        ",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let mut rows = match stmt.query(params![person_id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut note_ids: Vec<u64> = vec![];
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => note_ids.push(row),
                        Err(_) => return Err(DbOperationsError::RecordError),
                    },
                    None => break,
                },
                Err(_) => return Err(DbOperationsError::RecordError),
            }
        }

        if note_ids.is_empty() {
            return Ok(vec![]);
        }

        let vars = crate::helpers::repeat_vars(note_ids.len());
        let sql = format!(
            "SELECT * from notes WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = match conn.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let rows = match stmt.query_map(params_from_iter(note_ids.iter()), |row| {
            Ok(crate::entities::note::Note::new(
                row.get(0).unwrap(),
                crate::helpers::parse_from_str_ymd(
                    String::from(row.get::<usize, String>(1).unwrap_or_default()).as_str(),
                )
                .unwrap_or_default(),
                row.get(2).unwrap(),
                vec![],
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut notes = vec![];
        for note in rows {
            let note = match note {
                Ok(note) => note,
                Err(_) => return Err(DbOperationsError::RecordError),
            };
            notes.push(note);
        }

        Ok(notes)
    }

    pub fn get_reminders_by_person(
        conn: &Connection,
        person_id: u64,
    ) -> Result<Vec<crate::entities::reminder::Reminder>, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "SELECT
            reminder_id
        FROM
            people_reminders
        WHERE
            person_id = ?
            AND deleted = FALSE
        ",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let mut rows = match stmt.query(params![person_id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut reminder_ids: Vec<u64> = vec![];
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => reminder_ids.push(row),
                        Err(_) => return Err(DbOperationsError::RecordError),
                    },
                    None => break,
                },
                Err(_) => return Err(DbOperationsError::RecordError),
            }
        }

        if reminder_ids.is_empty() {
            return Ok(vec![]);
        }

        let vars = crate::helpers::repeat_vars(reminder_ids.len());
        let sql = format!(
            "SELECT * FROM reminders WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = match conn.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let rows = match stmt.query_map(params_from_iter(reminder_ids.iter()), |row| {
            Ok(crate::entities::reminder::Reminder::new(
                row.get(0).unwrap(),
                row.get(1).unwrap(),
                crate::helpers::parse_from_str_ymd(
                    String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                )
                .unwrap_or_default(),
                row.get(3).unwrap(),
                crate::entities::reminder::RecurringType::get_by_id(&conn, row.get(4).unwrap())
                    .unwrap(),
                vec![],
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut reminders = vec![];
        for reminder in rows {
            let reminder = match reminder {
                Ok(reminder) => reminder,
                Err(_) => return Err(DbOperationsError::RecordError),
            };
            reminders.push(reminder);
        }

        Ok(reminders)
    }

    pub fn get_contact_info_by_person(
        conn: &Connection,
        person_id: u64,
    ) -> Result<Vec<crate::entities::person::ContactInfo>, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "SELECT 
                * 
            FROM
                contact_info
            WHERE
                person_id = ?
                AND deleted = FALSE
            ",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let mut contact_info_vec: Vec<crate::entities::person::ContactInfo> = vec![];
        let rows = match stmt.query_map(params![person_id], |row| {
            let contact_info_type =
                crate::entities::person::ContactInfoType::get_by_id(&conn, row.get(2).unwrap())
                    .unwrap();
            let contact_info_details: String = row.get(3).unwrap();
            let contact_info = match contact_info_type {
                Some(ContactInfoType::Phone(_)) => ContactInfoType::Phone(contact_info_details),
                Some(ContactInfoType::WhatsApp(_)) => {
                    ContactInfoType::WhatsApp(contact_info_details)
                }
                Some(ContactInfoType::Email(_)) => ContactInfoType::Email(contact_info_details),
                None => panic!("Unknown contact info type {:#?}", contact_info_type),
            };

            Ok(crate::entities::person::ContactInfo::new(
                row.get(0).unwrap(),
                row.get(1).unwrap(),
                contact_info,
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        for contact_info in rows {
            let contact_info = match contact_info {
                Ok(contact_info) => contact_info,
                Err(_) => return Err(DbOperationsError::RecordError),
            };
            contact_info_vec.push(contact_info);
        }

        Ok(contact_info_vec)
    }

    pub fn get_activities_by_person(
        conn: &Connection,
        person_id: u64,
    ) -> Result<Vec<crate::entities::activity::Activity>, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "SELECT 
                activity_id 
            FROM
                people_activities
            WHERE
                person_id = ?
                AND deleted = FALSE
            ",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let mut rows = match stmt.query(params![person_id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut activity_ids: Vec<u64> = vec![];
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => activity_ids.push(row),
                        Err(_) => return Err(DbOperationsError::RecordError),
                    },
                    None => break,
                },
                Err(_) => return Err(DbOperationsError::RecordError),
            }
        }

        if activity_ids.is_empty() {
            return Ok(vec![]);
        }

        let vars = crate::helpers::repeat_vars(activity_ids.len());
        let sql = format!(
            "SELECT * FROM activities WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = match conn.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let rows = match stmt.query_map(params_from_iter(activity_ids.iter()), |row| {
            let activity_id = match row.get(0) {
                Ok(activity_id) => activity_id,
                Err(e) => panic!("{:#?}", e),
            };
            let people = match crate::db_helpers::get_people_by_activity(&conn, activity_id, false)
            {
                Ok(people) => people,
                Err(e) => panic!("{:#?}", e),
            };
            let activity_type = match crate::entities::activity::ActivityType::get_by_id(
                &conn,
                row.get(2).unwrap(),
            ) {
                Ok(activity_type) => match activity_type {
                    Some(activity_type) => activity_type,
                    None => panic!("Activity type cannot be None"),
                },
                Err(e) => panic!("{:#?}", e),
            };
            Ok(crate::entities::activity::Activity::new(
                activity_id,
                row.get(1).unwrap(),
                activity_type,
                crate::helpers::parse_from_str_ymd(
                    String::from(row.get::<usize, String>(3).unwrap_or_default()).as_str(),
                )
                .unwrap_or_default(),
                row.get(4).unwrap(),
                people,
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut activities = vec![];
        for activity in rows {
            let activity = match activity {
                Ok(activity) => activity,
                Err(_) => return Err(DbOperationsError::RecordError),
            };
            activities.push(activity);
        }

        Ok(activities)
    }

    // TODO remove duplication with similar functions
    pub fn get_people_by_reminder(
        conn: &Connection,
        reminder_id: u64,
    ) -> Result<Vec<crate::entities::person::Person>, DbOperationsError> {
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

        let mut rows = match stmt.query(params![reminder_id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut people_ids: Vec<u64> = vec![];
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => people_ids.push(row),
                        Err(_) => return Err(DbOperationsError::RecordError),
                    },
                    None => break,
                },
                Err(_) => return Err(DbOperationsError::RecordError),
            }
        }

        if people_ids.is_empty() {
            return Ok(vec![]);
        }

        let vars = crate::helpers::repeat_vars(people_ids.len());
        let sql = format!(
            "SELECT * FROM people WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = match conn.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let rows = match stmt.query_map(params_from_iter(people_ids.iter()), |row| {
            let person_id = row.get(0).unwrap();
            // TODO handle this properly
            let notes = match crate::db::db_helpers::get_notes_by_person(&conn, person_id) {
                Ok(notes) => notes,
                Err(e) => panic!("{:#?}", e),
            };
            let reminders = match crate::db::db_helpers::get_reminders_by_person(&conn, person_id) {
                Ok(reminders) => reminders,
                Err(e) => panic!("{:#?}", e),
            };
            let contact_info =
                match crate::db::db_helpers::get_contact_info_by_person(&conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => panic!("{:#?}", e),
                };
            let activities = match crate::db::db_helpers::get_activities_by_person(&conn, person_id)
            {
                Ok(activities) => activities,
                Err(e) => panic!("{:#?}", e),
            };
            Ok(crate::entities::person::Person {
                id: person_id,
                name: row.get(1).unwrap(),
                birthday: Some(
                    crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                ),
                contact_info: contact_info,
                activities: activities,
                reminders: reminders,
                notes: notes,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut activities = vec![];
        for activity in rows {
            let activity = match activity {
                Ok(activity) => activity,
                Err(_) => return Err(DbOperationsError::RecordError),
            };
            activities.push(activity);
        }

        Ok(activities)
    }

    // TODO remove duplication with similar functions
    pub fn get_people_by_activity(
        conn: &Connection,
        activity_id: u64,
        recurse: bool,
    ) -> Result<Vec<crate::entities::person::Person>, DbOperationsError> {
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

        let mut rows = match stmt.query(params![activity_id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut people_ids: Vec<u64> = vec![];
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => people_ids.push(row),
                        Err(_) => return Err(DbOperationsError::RecordError),
                    },
                    None => break,
                },
                Err(_) => return Err(DbOperationsError::RecordError),
            }
        }

        if people_ids.is_empty() {
            return Ok(vec![]);
        }

        let vars = crate::helpers::repeat_vars(people_ids.len());
        let sql = format!(
            "SELECT * FROM people WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = conn.prepare(&sql).expect("Invalid SQL statement");

        let rows = match stmt.query_map(params_from_iter(people_ids.iter()), |row| {
            let person_id = row.get(0).unwrap();
            // TODO handle this properly
            let notes = match crate::db::db_helpers::get_notes_by_person(&conn, person_id) {
                Ok(notes) => notes,
                Err(e) => panic!("{:#?}", e),
            };
            let reminders = match crate::db::db_helpers::get_reminders_by_person(&conn, person_id) {
                Ok(reminders) => reminders,
                Err(e) => panic!("{:#?}", e),
            };
            let contact_info =
                match crate::db::db_helpers::get_contact_info_by_person(&conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => panic!("{:#?}", e),
                };
            let activities = match crate::db::db_helpers::get_activities_by_person(&conn, person_id)
            {
                Ok(activities) => activities,
                Err(e) => panic!("{:#?}", e),
            };
            Ok(crate::entities::person::Person {
                id: person_id,
                name: row.get(1).unwrap(),
                birthday: Some(
                    crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                ),
                contact_info: contact_info,
                activities: if recurse { activities } else { vec![] },
                reminders: reminders,
                notes: notes,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut people = vec![];
        for person in rows {
            let person = match person {
                Ok(person) => person,
                Err(_) => return Err(DbOperationsError::RecordError),
            };
            people.push(person);
        }

        Ok(people)
    }

    pub fn get_people_by_note(
        conn: &Connection,
        note_id: u64,
    ) -> Result<Vec<crate::entities::person::Person>, DbOperationsError> {
        let mut stmt = match conn.prepare(
            "SELECT
                        person_id
                    FROM
                        people_notes
                    WHERE
                        note_id = ?
                        AND deleted = FALSE
                    ",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let mut rows = match stmt.query(params![note_id]) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };
        let mut people_ids: Vec<u64> = vec![];
        loop {
            match rows.next() {
                Ok(row) => match row {
                    Some(row) => match row.get(0) {
                        Ok(row) => people_ids.push(row),
                        Err(_) => return Err(DbOperationsError::RecordError),
                    },
                    None => break,
                },
                Err(_) => return Err(DbOperationsError::RecordError),
            }
        }

        if people_ids.is_empty() {
            return Ok(vec![]);
        }

        let vars = crate::helpers::repeat_vars(people_ids.len());
        let sql = format!(
            "SELECT * FROM people WHERE id IN ({}) AND deleted = FALSE",
            vars
        );
        let mut stmt = match conn.prepare(&sql) {
            Ok(stmt) => stmt,
            Err(_) => return Err(DbOperationsError::InvalidStatement),
        };

        let rows = match stmt.query_map(params_from_iter(people_ids.iter()), |row| {
            let person_id = row.get(0).unwrap();
            // TODO handle this properly
            let notes = match crate::db::db_helpers::get_notes_by_person(&conn, person_id) {
                Ok(notes) => notes,
                Err(e) => panic!("{:#?}", e),
            };
            let reminders = match crate::db::db_helpers::get_reminders_by_person(&conn, person_id) {
                Ok(reminders) => reminders,
                Err(e) => panic!("{:#?}", e),
            };
            let contact_info =
                match crate::db::db_helpers::get_contact_info_by_person(&conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => panic!("{:#?}", e),
                };
            let activities = match crate::db::db_helpers::get_activities_by_person(&conn, person_id)
            {
                Ok(activities) => activities,
                Err(e) => panic!("{:#?}", e),
            };
            Ok(crate::entities::person::Person {
                id: person_id,
                name: row.get(1).unwrap(),
                birthday: Some(
                    crate::helpers::parse_from_str_ymd(
                        String::from(row.get::<usize, String>(2).unwrap_or_default()).as_str(),
                    )
                    .unwrap_or_default(),
                ),
                contact_info: contact_info,
                activities: activities,
                reminders: reminders,
                notes: notes,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Err(DbOperationsError::QueryError),
        };

        let mut notes = vec![];
        for note in rows {
            let note = match note {
                Ok(note) => note,
                Err(_) => return Err(DbOperationsError::RecordError),
            };
            notes.push(note);
        }

        Ok(notes)
    }

    pub fn init_db(conn: &Connection) -> Result<(), DbOperationsError> {
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
            let mut stmt = match conn.prepare(query) {
                Ok(stmt) => stmt,
                Err(_) => return Err(DbOperationsError::InvalidStatement),
            };
            match stmt.execute(params![]) {
                // TODO Improve message
                Ok(_) => println!("Database table created"),
                Err(error) => {
                    println!("Error creating database tables: {}", error);
                    return Err(DbOperationsError::GenericError);
                }
            };
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
            ('OneTime', FALSE),
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
            let mut stmt = match conn.prepare(query) {
                Ok(stmt) => stmt,
                Err(_) => return Err(DbOperationsError::InvalidStatement),
            };
            match stmt.execute(params![]) {
                // TODO Improve message
                Ok(_) => println!("Database table populated"),
                Err(error) => {
                    println!("Error populating database tables: {}", error);
                    return Err(DbOperationsError::GenericError);
                }
            };
        }
        Ok(())
    }
}
