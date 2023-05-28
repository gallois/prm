use crate::entities::activity::{Activity, ACTIVITY_TEMPLATE};
use crate::helpers::ActivityVars;
use std::collections::HashMap;
use strfmt::strfmt;

use snafu::prelude::*;

// TODO merge errors
#[derive(Debug, Snafu)]
pub enum ParseError {
    FieldError,
    FormatError,
    #[snafu(display("Error while parsing activity: {}", activity))]
    ActivityParseError {
        activity: String,
    },
}

pub fn populate_activity_vars(vars: HashMap<String, String>) -> Result<ActivityVars, ParseError> {
    let edited = edit::edit(strfmt(ACTIVITY_TEMPLATE, &vars).unwrap()).unwrap();
    let (n, d, t, c, p) = match Activity::parse_from_editor(edited.as_str()) {
        Ok((name, date, activity_type, content, people)) => {
            (name, date, activity_type, content, people)
        }
        Err(_) => return ActivityParseSnafu { activity: edited }.fail(),
    };

    return Ok(ActivityVars {
        name: n,
        date: d.unwrap(),
        activity_type: t.unwrap(),
        content: c.unwrap(),
        people: p,
    });
}
