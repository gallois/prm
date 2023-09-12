use snafu::prelude::*;
use std::collections::HashMap;

pub mod add;
pub mod edit;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum CliError {
    #[snafu(display("Invalid birthday: {}", birthday))]
    BirthdayParse { birthday: String },
    #[snafu(display("Invalid contact info: {}", contact_info))]
    ContactInfoParse { contact_info: String },
    #[snafu(display("Invalid activity type: {}", activity_type))]
    ActivityTypeParse { activity_type: String },
    #[snafu(display("Invalid date: {}", date))]
    DateParse { date: String },
    #[snafu(display("Invalid recurring type: {}", recurring_type))]
    RecurringTypeParse { recurring_type: String },
    #[snafu(display("Error parsing {} from editor", entity))]
    EditorParse { entity: String },
    #[snafu(display("Error adding {}", entity))]
    Add { entity: String },
    #[snafu(display("Entity error {}", entity))]
    Entity { entity: String },
    #[snafu(display("Error editing {}", entity))]
    Edit { entity: String },
    #[snafu(display("Entity not found {} for id {}", entity, id))]
    NotFound { entity: String, id: u64 },
    #[snafu(display("Unexpected missing field {}: {}", entity, field))]
    MissingField { entity: String, field: String },
    #[snafu(display("Failed to apply string template {}: {:#?}", template, vars))]
    Template {
        template: String,
        vars: HashMap<String, String>,
    },
}
