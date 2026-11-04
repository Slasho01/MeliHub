use chrono::{NaiveDateTime, Utc};

pub fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}
