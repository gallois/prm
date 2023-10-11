use rusqlite::{params, Connection};

pub mod db_interface {
    use crate::db::Connection;

    #[derive(Debug)]
    pub enum DbOperationsError {
        DuplicateEntry,
        GenericError,
        InvalidStatement {
            sqlite_error: rusqlite::Error,
        },
        QueryError,
        RecordError {
            sqlite_error: Option<rusqlite::Error>,
            strum_error: Option<strum::ParseError>,
        },
        InitialisationError {
            action: String,
        },
        UnexpectedMultipleEntries,
    }

    pub trait DbOperations {
        fn add(&self, conn: &Connection) -> Result<&Self, DbOperationsError>;
        fn remove(&self, conn: &Connection) -> Result<&Self, DbOperationsError>;
        fn save(&self, conn: &Connection) -> Result<&Self, DbOperationsError>;
        fn get_by_id(
            conn: &Connection,
            id: u64,
        ) -> Result<Option<crate::entities::Entities>, DbOperationsError>;
        fn get_all(conn: &Connection) -> Result<Vec<Box<Self>>, DbOperationsError>;
    }
}

pub mod db_helpers {
    use crate::db::{params, Connection};
    use crate::db_interface::DbOperationsError;

    pub mod notes {
        use rusqlite::{params, params_from_iter, Connection};

        use crate::db_interface::DbOperationsError;

        pub fn get_by_person(
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
            AND deleted = 0
        ",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
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
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        },
                        None => break,
                    },
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                }
            }

            if note_ids.is_empty() {
                return Ok(vec![]);
            }

            let vars = crate::helpers::repeat_vars(note_ids.len());
            let sql = format!("SELECT * FROM notes WHERE id IN ({}) AND deleted = 0", vars);
            let mut stmt = match conn.prepare(&sql) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let rows = match stmt.query_map(params_from_iter(note_ids.iter()), |row| {
                Ok(crate::entities::note::Note::new(
                    row.get(0)?,
                    crate::helpers::parse_from_str_ymd(
                        row.get::<usize, String>(1).unwrap_or_default().as_str(),
                    )
                    .unwrap_or_default(),
                    row.get(2)?,
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
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                };
                notes.push(note);
            }

            Ok(notes)
        }
    }

    pub mod reminders {
        use rusqlite::{params, params_from_iter, Connection};

        use crate::{db_interface::DbOperationsError, entities::reminder::RecurringType};

        pub fn get_by_person(
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
            AND deleted = 0
        ",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
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
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        },
                        None => break,
                    },
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
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
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let rows = match stmt.query_map(params_from_iter(reminder_ids.iter()), |row| {
                let recurring_type = match RecurringType::get_by_id(conn, row.get(4)?) {
                    Ok(recurring_type) => match recurring_type {
                        Some(recurring_type) => recurring_type,
                        None => panic!("Recurring Type cannot be None"),
                    },
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                Ok(crate::entities::reminder::Reminder::new(
                    row.get(0)?,
                    row.get(1)?,
                    crate::helpers::parse_from_str_ymd(
                        row.get::<usize, String>(2).unwrap_or_default().as_str(),
                    )
                    .unwrap_or_default(),
                    row.get(3)?,
                    recurring_type,
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
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                };
                reminders.push(reminder);
            }

            Ok(reminders)
        }
    }

    pub mod contact_info {
        use rusqlite::{params, Connection};

        use crate::{db_interface::DbOperationsError, entities::person::ContactInfoType};

        pub fn get_by_person(
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
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let mut contact_info_vec: Vec<crate::entities::person::ContactInfo> = vec![];
            let rows = match stmt.query_map(params![person_id], |row| {
                let contact_info_type = match ContactInfoType::get_by_id(conn, row.get(2)?) {
                    Ok(contact_info_type) => match contact_info_type {
                        Some(contact_info_type) => contact_info_type,
                        None => panic!("Contact Info Type cannot be None"),
                    },
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };

                let contact_info_details: String = row.get(3)?;
                let contact_info = match contact_info_type {
                    ContactInfoType::Phone(_) => ContactInfoType::Phone(contact_info_details),
                    ContactInfoType::WhatsApp(_) => ContactInfoType::WhatsApp(contact_info_details),
                    ContactInfoType::Email(_) => ContactInfoType::Email(contact_info_details),
                };

                Ok(crate::entities::person::ContactInfo::new(
                    row.get(0)?,
                    row.get(1)?,
                    contact_info,
                ))
            }) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            for contact_info in rows {
                let contact_info = match contact_info {
                    Ok(contact_info) => contact_info,
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                };
                contact_info_vec.push(contact_info);
            }

            Ok(contact_info_vec)
        }
    }

    pub mod activities {
        use rusqlite::{params, params_from_iter, Connection};

        use crate::{db_interface::DbOperationsError, entities::activity::Activity};

        pub fn get_by_person(
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
                AND deleted = 0
            ",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
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
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        },
                        None => break,
                    },
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                }
            }

            if activity_ids.is_empty() {
                return Ok(vec![]);
            }

            let vars = crate::helpers::repeat_vars(activity_ids.len());
            let sql = format!(
                "SELECT * FROM activities WHERE id IN ({}) AND deleted = 0",
                vars
            );
            let mut stmt = match conn.prepare(&sql) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let rows = match stmt.query_map(params_from_iter(activity_ids.iter()), |row| {
                let activity_id = row.get(0)?;
                let people =
                    match crate::db_helpers::people::get_by_activity(conn, activity_id, false) {
                        Ok(people) => people,
                        Err(e) => {
                            let sqlite_error = match e {
                                DbOperationsError::InvalidStatement { sqlite_error } => {
                                    sqlite_error
                                }
                                other => panic!("Unexpected error type: {:#?}", other),
                            };
                            return Err(sqlite_error);
                        }
                    };
                let activity_type =
                    match crate::entities::activity::ActivityType::get_by_id(conn, row.get(2)?) {
                        Ok(activity_type) => match activity_type {
                            Some(activity_type) => activity_type,
                            None => panic!("Activity type cannot be None"),
                        },
                        Err(e) => {
                            let sqlite_error = match e {
                                DbOperationsError::InvalidStatement { sqlite_error } => {
                                    sqlite_error
                                }
                                other => panic!("Unexpected error type: {:#?}", other),
                            };
                            return Err(sqlite_error);
                        }
                    };
                Ok(crate::entities::activity::Activity::new(
                    activity_id,
                    row.get(1)?,
                    activity_type,
                    crate::helpers::parse_from_str_ymd(
                        row.get::<usize, String>(3).unwrap_or_default().as_str(),
                    )
                    .unwrap_or_default(),
                    row.get(4)?,
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
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                };
                activities.push(activity);
            }

            Ok(activities)
        }

        pub fn get_by_name(
            conn: &Connection,
            name: String,
            person: Option<String>,
        ) -> Result<Vec<Activity>, DbOperationsError> {
            let mut activities: Vec<Activity> = vec![];
            let mut stmt = match conn.prepare(
                "
                    SELECT 
                        * 
                    FROM 
                        activities 
                    WHERE 
                        name LIKE '%' || ?1 || '%' AND 
                        deleted = 0 
                    COLLATE NOCASE",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            let mut rows = match stmt.query(params![name]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };
            loop {
                match rows.next() {
                    Ok(row) => match row {
                        Some(row) => {
                            let activity = crate::entities::activity::Activity::build_from_sql(
                                conn,
                                row.get(0),
                                row.get(1),
                                row.get(2),
                                row.get::<usize, String>(3),
                                row.get(4),
                            )?;
                            if let Some(person) = person.clone() {
                                let people_name: Vec<String> =
                                    activity.people.iter().map(|p| p.name.to_owned()).collect();
                                if people_name.contains(&person) {
                                    activities.push(activity);
                                }
                            } else {
                                activities.push(activity);
                            }
                        }
                        None => return Ok(activities),
                    },
                    Err(_) => return Err(DbOperationsError::GenericError),
                }
            }
        }

        pub fn get_by_person_name(
            conn: &Connection,
            person: String,
        ) -> Result<Vec<Activity>, DbOperationsError> {
            let mut activities: Vec<Activity> = vec![];
            let mut stmt = match conn.prepare(
                "
                    SELECT 
                        id 
                    FROM 
                        people 
                    WHERE 
                        name = ?1 AND 
                        deleted = 0 
                    COLLATE NOCASE",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            let mut rows = match stmt.query(params![person]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };
            let person_id: u64;
            match rows.next() {
                Ok(row) => {
                    if let Some(row) = row {
                        person_id = match row.get(0) {
                            Ok(person_id) => person_id,
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        };
                        let activity_ids = get_ids_by_person_id(conn, person_id)?;

                        let vars = crate::helpers::repeat_vars(activity_ids.len());
                        let sql = format!(
                            "SELECT 
                                * 
                            FROM 
                                activities 
                            WHERE 
                                id IN ({}) AND 
                            deleted = 0",
                            vars
                        );
                        let mut stmt = match conn.prepare(&sql) {
                            Ok(stmt) => stmt,
                            Err(e) => {
                                return Err(DbOperationsError::InvalidStatement { sqlite_error: e })
                            }
                        };

                        let mut rows = match stmt.query(params_from_iter(activity_ids.iter())) {
                            Ok(rows) => rows,
                            Err(_) => return Err(DbOperationsError::QueryError),
                        };

                        loop {
                            match rows.next() {
                                Ok(row) => match row {
                                    Some(row) => {
                                        let activity =
                                            crate::entities::activity::Activity::build_from_sql(
                                                conn,
                                                row.get(0),
                                                row.get(1),
                                                row.get(2),
                                                row.get::<usize, String>(3),
                                                row.get(4),
                                            )?;
                                        activities.push(activity);
                                    }
                                    None => break,
                                },
                                Err(_) => return Err(DbOperationsError::GenericError),
                            }
                        }
                    }
                }
                Err(_) => return Err(DbOperationsError::GenericError),
            }

            Ok(activities)
        }

        pub fn get_ids_by_person_id(
            conn: &Connection,
            person_id: u64,
        ) -> Result<Vec<u8>, DbOperationsError> {
            let mut ids: Vec<u8> = vec![];
            let mut stmt = match conn.prepare(
                "
                    SELECT 
                        activity_id 
                    FROM 
                        people_activities 
                    WHERE 
                        person_id = ?1 AND 
                    deleted = 0",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            let mut rows = match stmt.query(params![person_id]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            loop {
                match rows.next() {
                    Ok(row) => match row {
                        Some(row) => {
                            match row.get(0) {
                                Ok(id) => ids.push(id),
                                Err(e) => {
                                    return Err(DbOperationsError::RecordError {
                                        sqlite_error: Some(e),
                                        strum_error: None,
                                    })
                                }
                            };
                        }
                        None => break,
                    },
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                }
            }
            Ok(ids)
        }

        pub fn get_by_content(
            conn: &Connection,
            content: String,
        ) -> Result<Vec<Activity>, DbOperationsError> {
            let mut activities: Vec<Activity> = vec![];
            let mut stmt = match conn.prepare(
                "
                    SELECT 
                        * 
                    FROM 
                        activities 
                    WHERE 
                        content LIKE '%' || ?1 || '%' AND 
                        deleted = 0 
                    COLLATE NOCASE",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            let mut rows = match stmt.query(params![content]) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };
            loop {
                match rows.next() {
                    Ok(row) => match row {
                        Some(row) => {
                            let activity = crate::entities::activity::Activity::build_from_sql(
                                conn,
                                row.get(0),
                                row.get(1),
                                row.get(2),
                                row.get::<usize, String>(3),
                                row.get(4),
                            )?;
                            activities.push(activity);
                        }
                        None => return Ok(activities),
                    },
                    Err(_) => return Err(DbOperationsError::GenericError),
                }
            }
        }
    }

    pub mod people {
        use rusqlite::{params, params_from_iter, Connection};

        use crate::db_interface::DbOperationsError;

        use super::{activities, contact_info, notes, reminders};

        pub fn get_by_reminder(
            conn: &Connection,
            reminder_id: u64,
        ) -> Result<Vec<crate::entities::person::Person>, DbOperationsError> {
            let mut stmt = match conn.prepare(
                "SELECT
                        person_id
                    FROM
                        people_reminders
                    WHERE
                        reminder_id = ?
                        AND deleted = 0
            ",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

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
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        },
                        None => break,
                    },
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                }
            }

            if people_ids.is_empty() {
                return Ok(vec![]);
            }

            let vars = crate::helpers::repeat_vars(people_ids.len());
            let sql = format!(
                "SELECT * FROM people WHERE id IN ({}) AND deleted = 0",
                vars
            );
            let mut stmt = match conn.prepare(&sql) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let rows = match stmt.query_map(params_from_iter(people_ids.iter()), |row| {
                let person_id = row.get(0)?;
                let notes = match notes::get_by_person(conn, person_id) {
                    Ok(notes) => notes,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let reminders = match reminders::get_by_person(conn, person_id) {
                    Ok(reminders) => reminders,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let contact_info = match contact_info::get_by_person(conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let activities = match activities::get_by_person(conn, person_id) {
                    Ok(activities) => activities,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                Ok(crate::entities::person::Person {
                    id: person_id,
                    name: row.get(1)?,
                    birthday: Some(
                        crate::helpers::parse_from_str_ymd(
                            row.get::<usize, String>(2).unwrap_or_default().as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    contact_info,
                    activities,
                    reminders,
                    notes,
                })
            }) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            let mut activities = vec![];
            for activity in rows {
                let activity = match activity {
                    Ok(activity) => activity,
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                };
                activities.push(activity);
            }

            Ok(activities)
        }

        pub fn get_by_activity(
            conn: &Connection,
            activity_id: u64,
            recurse: bool,
        ) -> Result<Vec<crate::entities::person::Person>, DbOperationsError> {
            let mut stmt = match conn.prepare(
                "SELECT
                        person_id
                    FROM
                        people_activities
                    WHERE
                        activity_id = ?
                        AND deleted = 0
            ",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

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
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        },
                        None => break,
                    },
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                }
            }

            if people_ids.is_empty() {
                return Ok(vec![]);
            }

            let vars = crate::helpers::repeat_vars(people_ids.len());
            let sql = format!(
                "SELECT * FROM people WHERE id IN ({}) AND deleted = 0",
                vars
            );
            let mut stmt = match conn.prepare(&sql) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let rows = match stmt.query_map(params_from_iter(people_ids.iter()), |row| {
                let person_id = row.get(0)?;
                let notes = match notes::get_by_person(conn, person_id) {
                    Ok(notes) => notes,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let reminders = match reminders::get_by_person(conn, person_id) {
                    Ok(reminders) => reminders,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let contact_info = match contact_info::get_by_person(conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let mut activities: Vec<crate::entities::activity::Activity> = vec![];
                if recurse {
                    activities = match activities::get_by_person(conn, person_id) {
                        Ok(activities) => activities,
                        Err(e) => {
                            let sqlite_error = match e {
                                DbOperationsError::InvalidStatement { sqlite_error } => {
                                    sqlite_error
                                }
                                other => panic!("Unexpected error type: {:#?}", other),
                            };
                            return Err(sqlite_error);
                        }
                    };
                }
                Ok(crate::entities::person::Person {
                    id: person_id,
                    name: row.get(1)?,
                    birthday: Some(
                        crate::helpers::parse_from_str_ymd(
                            row.get::<usize, String>(2).unwrap_or_default().as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    contact_info,
                    activities,
                    reminders,
                    notes,
                })
            }) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            let mut people = vec![];
            for person in rows {
                let person = match person {
                    Ok(person) => person,
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                };
                people.push(person);
            }

            Ok(people)
        }

        pub fn get_by_note(
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
                        AND deleted = 0
                    ",
            ) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
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
                            Err(e) => {
                                return Err(DbOperationsError::RecordError {
                                    sqlite_error: Some(e),
                                    strum_error: None,
                                })
                            }
                        },
                        None => break,
                    },
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                }
            }

            if people_ids.is_empty() {
                return Ok(vec![]);
            }

            let vars = crate::helpers::repeat_vars(people_ids.len());
            let sql = format!(
                "SELECT * FROM people WHERE id IN ({}) AND deleted = 0",
                vars
            );
            let mut stmt = match conn.prepare(&sql) {
                Ok(stmt) => stmt,
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };

            let rows = match stmt.query_map(params_from_iter(people_ids.iter()), |row| {
                let person_id = row.get(0)?;
                let notes = match notes::get_by_person(conn, person_id) {
                    Ok(notes) => notes,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let reminders = match reminders::get_by_person(conn, person_id) {
                    Ok(reminders) => reminders,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let contact_info = match contact_info::get_by_person(conn, person_id) {
                    Ok(contact_info) => contact_info,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                let activities = match activities::get_by_person(conn, person_id) {
                    Ok(activities) => activities,
                    Err(e) => {
                        let sqlite_error = match e {
                            DbOperationsError::InvalidStatement { sqlite_error } => sqlite_error,
                            other => panic!("Unexpected error type: {:#?}", other),
                        };
                        return Err(sqlite_error);
                    }
                };
                Ok(crate::entities::person::Person {
                    id: person_id,
                    name: row.get(1)?,
                    birthday: Some(
                        crate::helpers::parse_from_str_ymd(
                            row.get::<usize, String>(2).unwrap_or_default().as_str(),
                        )
                        .unwrap_or_default(),
                    ),
                    contact_info,
                    activities,
                    reminders,
                    notes,
                })
            }) {
                Ok(rows) => rows,
                Err(_) => return Err(DbOperationsError::QueryError),
            };

            let mut notes = vec![];
            for note in rows {
                let note = match note {
                    Ok(note) => note,
                    Err(e) => {
                        return Err(DbOperationsError::RecordError {
                            sqlite_error: Some(e),
                            strum_error: None,
                        })
                    }
                };
                notes.push(note);
            }

            Ok(notes)
        }
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
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            match stmt.execute(params![]) {
                Ok(_) => println!("Database table created"),
                Err(error) => {
                    println!("Error creating database tables: {}", error);
                    return Err(DbOperationsError::InitialisationError {
                        action: String::from("create"),
                    });
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
                Err(e) => return Err(DbOperationsError::InvalidStatement { sqlite_error: e }),
            };
            match stmt.execute(params![]) {
                Ok(_) => println!("Database table populated"),
                Err(error) => {
                    println!("Error populating database tables: {}", error);
                    return Err(DbOperationsError::InitialisationError {
                        action: String::from("insert"),
                    });
                }
            };
        }
        Ok(())
    }
}
