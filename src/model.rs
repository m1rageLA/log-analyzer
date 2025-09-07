use chrono::{NaiveDateTime};
use serde::{Serialize, Deserialize};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Level { Info, Warning, Error }


#[derive(Clone, Debug)]
pub struct LogEntry {
pub ts: NaiveDateTime,
pub level: Level,
pub message: String,
}