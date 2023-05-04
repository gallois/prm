// Helper function to return a comma-separated sequence of `?`.
// - `repeat_vars(0) => panic!(...)`
// - `repeat_vars(1) => "?"`
// - `repeat_vars(2) => "?,?"`
// - `repeat_vars(3) => "?,?,?"`
// - ...
pub fn repeat_vars(count: usize) -> String {
    assert_ne!(count, 0);
    let mut s = "?,".repeat(count);
    // Remove trailing comma
    s.pop();
    s
}

pub fn parse_from_str_ymd(date: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
}

pub fn parse_from_str_md(date: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
    parse_from_str_ymd(format!("1-{}", date).as_ref())
}

pub fn unwrap_arg_or_empty_string(arg: Option<String>) -> String {
    arg.unwrap_or("".to_string())
}

pub struct ActivityVars {
    pub name: String,
    pub date: String,
    pub activity_type: String,
    pub content: String,
    pub people: Vec<String>,
}
