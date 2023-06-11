use crate::entities::activity::{Activity, ACTIVITY_TEMPLATE};
use crate::helpers::ActivityVars;
use std::collections::HashMap;
use strfmt::strfmt;

use snafu::prelude::*;

// TODO merge errors
#[derive(Debug, Snafu)]
pub enum ParseError {
    #[snafu(display("Error parsing field: {}", field))]
    FieldError {
        field: String,
    },
    FormatError,
    TemplateError,
    EditorError,
    #[snafu(display("Error while parsing activity: {}", activity))]
    ActivityParseError {
        activity: String,
    },
}

pub fn populate_activity_vars(vars: HashMap<String, String>) -> Result<ActivityVars, ParseError> {
    let activities_str = match strfmt(ACTIVITY_TEMPLATE, &vars) {
        Ok(activities_str) => activities_str,
        Err(_) => return Err(ParseError::TemplateError),
    };
    let edited = match edit::edit(&activities_str) {
        Ok(edited) => edited,
        Err(_) => return Err(ParseError::EditorError),
    };
    let (n, d, t, c, p) = match Activity::parse_from_editor(edited.as_str()) {
        Ok((name, date, activity_type, content, people)) => {
            (name, date, activity_type, content, people)
        }
        Err(_) => return ActivityParseSnafu { activity: edited }.fail(),
    };

    let date = match d {
        Some(date) => date,
        None => return FieldSnafu { field: "date" }.fail(),
    };
    let activity_type = match t {
        Some(activity_type) => activity_type,
        None => {
            return FieldSnafu {
                field: "activity_type",
            }
            .fail()
        }
    };
    let content = match c {
        Some(content) => content,
        None => return FieldSnafu { field: "content" }.fail(),
    };

    return Ok(ActivityVars {
        name: n,
        date,
        activity_type,
        content,
        people: p,
    });
}
