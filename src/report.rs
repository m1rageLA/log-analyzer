use crate::analyze::{Analyzer, Filters};
use crate::model::Level;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;


#[derive(Serialize, Deserialize)]
pub struct Counts { pub info: u64, pub warning: u64, pub error: u64 }


#[derive(Serialize, Deserialize)]
pub struct JsonSummary {
pub total_entries: u64,
pub counts: Counts,
pub malformed_lines: u64,
pub first_log: Option<String>,
pub last_log: Option<String>,
pub common_errors: Vec<(String, u64)>,
/// timeline buckets in RFC3339-like naive format for portability
pub timeline: Vec<(String, u64)>,
}


pub fn build_summary(an: &Analyzer, _filters: &Filters) -> JsonSummary {
let total = an.info + an.warning + an.error;


// Sort error messages by freq desc, then message asc, take top 10
let mut errs: Vec<(String, u64)> = an.error_messages.iter().map(|(k,v)| (k.clone(), *v)).collect();
errs.sort_by(|a,b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
errs.truncate(10);


let timeline: Vec<(String, u64)> = an.timeline.iter()
.map(|(k,v)| (format!("{}", k.format("%Y-%m-%d %H:%M:%S")), *v)).collect();


JsonSummary {
total_entries: total,
counts: Counts { info: an.info, warning: an.warning, error: an.error },
malformed_lines: an.malformed_lines,
first_log: an.first.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
last_log: an.last.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()),
common_errors: errs,
timeline,
}
}