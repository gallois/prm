use crate::helpers::ActivityVars;
use crate::{Activity, ACTIVITY_TEMPLATE};
use std::collections::HashMap;
use strfmt::strfmt;

pub fn populate_activity_vars(vars: HashMap<String, String>) -> ActivityVars {
    let edited = edit::edit(strfmt(ACTIVITY_TEMPLATE, &vars).unwrap()).unwrap();
    let (n, d, t, c, p) = match Activity::parse_from_editor(edited.as_str()) {
        Ok((name, date, activity_type, content, people)) => {
            (name, date, activity_type, content, people)
        }
        Err(_) => panic!("Error parsing activity"),
    };

    return ActivityVars {
        name: n,
        date: d.unwrap(),
        activity_type: t.unwrap(),
        content: c.unwrap(),
        people: p,
    };
}
