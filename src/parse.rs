use crate::model::{Level, LogEntry};
use chrono::{NaiveDateTime, Datelike, Local};
use regex::Regex;

pub trait LogParser {
    /// Parse a single line into an optional LogEntry
    fn parse_line(&mut self, line: &str) -> Result<Option<LogEntry>, String>;
}

/// Regex-based parser that tries multiple common patterns.
pub struct DefaultLogParser {
    patterns: Vec<Pattern>,
}

struct Pattern { re: Regex, ts_group: &'static str, level_group: &'static str, msg_group: &'static str }

impl DefaultLogParser {
    pub fn new() -> Self {
        // Pattern 1: `2025-09-05 14:32:10,123 INFO Message...`
        let p1 = Pattern {
            re: Regex::new(r"^(?P<ts>\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}:\d{2}(?:[.,]\d{1,6})?)\s+(?P<level>INFO|ERROR|WARNING|WARN)\s+(?P<msg>.*)$").unwrap(),
            ts_group: "ts", level_group: "level", msg_group: "msg"
        };
        // Pattern 2: `2025-09-05T14:32:10Z [WARNING] Message...` (ignores TZ)
        let p2 = Pattern {
            re: Regex::new(r"^(?P<ts>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)\s*\[(?P<level>INFO|ERROR|WARNING|WARN)\]\s*(?P<msg>.*)$").unwrap(),
            ts_group: "ts", level_group: "level", msg_group: "msg"
        };
        // Pattern 3: `Sep  5 14:32:10 host app[123]: [ERROR] Message...` (assume current year)
        let p3 = Pattern {
            re: Regex::new(r"^(?P<mon>Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(?P<day>\d{1,2})\s+(?P<time>\d{2}:\d{2}:\d{2}).*?(?P<level>INFO|ERROR|WARNING|WARN|\[INFO\]|\[ERROR\]|\[WARNING\]|\[WARN\]).*?\s(?P<msg>[^\r\n]*)$").unwrap(),
            ts_group: "time", level_group: "level", msg_group: "msg"
        };
        Self { patterns: vec![p1, p2, p3] }
    }
}

fn parse_ts(ts: &str) -> Option<NaiveDateTime> {
    // Try a bunch of formats
    let fmts = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
    ];
    for f in fmts { if let Ok(dt) = NaiveDateTime::parse_from_str(ts, f) { return Some(dt); } }
    // Maybe there is a trailing timezone indicator – strip it and retry
    if let Some((head, _)) = ts.split_once('Z') { if let Ok(dt) = NaiveDateTime::parse_from_str(head, "%Y-%m-%dT%H:%M:%S%.f") { return Some(dt); } }
    // Unhandled
    None
}

fn mon_to_num(mon: &str) -> u32 {
    match mon {"Jan"=>1,"Feb"=>2,"Mar"=>3,"Apr"=>4,"May"=>5,"Jun"=>6,"Jul"=>7,"Aug"=>8,"Sep"=>9,"Oct"=>10,"Nov"=>11,"Dec"=>12,_=>1}
}

fn parse_level(s: &str) -> Level {
    match s.trim_matches(['[',']']).to_ascii_uppercase().as_str() {
        "ERROR" => Level::Error,
        "WARNING" | "WARN" => Level::Warning,
        _ => Level::Info,
    }
}

impl LogParser for DefaultLogParser {
    fn parse_line(&mut self, line: &str) -> Result<Option<LogEntry>, String> {
        for p in &self.patterns {
            if let Some(caps) = p.re.captures(line) {
                // Special handling for pattern 3 (month/day format)
                if p.re.as_str().starts_with("^(?P<mon>") {
                    let mon = caps.name("mon").unwrap().as_str();
                    let day: u32 = caps.name("day").unwrap().as_str().parse().unwrap_or(1);
                    let time = caps.name("time").unwrap().as_str();
                    let year = Local::now().year();
                    let ts_str = format!("{year}-{m:02}-{d:02} {time}", m = mon_to_num(mon), d = day);
                    if let Ok(ts) = NaiveDateTime::parse_from_str(&ts_str, "%Y-%m-%d %H:%M:%S") {
                        return Ok(Some(LogEntry { ts, level: parse_level(caps.name(p.level_group).unwrap().as_str()), message: caps.name(p.msg_group).unwrap().as_str().to_string() }));
                    } else { return Err(format!("Failed to parse datetime: {ts_str}")); }
                }

                let ts_raw = caps.name(p.ts_group).unwrap().as_str();
                if let Some(ts) = parse_ts(ts_raw) {
                    let level = parse_level(caps.name(p.level_group).unwrap().as_str());
                    let message = caps.name(p.msg_group).unwrap().as_str().to_string();
                    return Ok(Some(LogEntry { ts, level, message }));
                } else {
                    return Err(format!("Could not parse timestamp: {ts_raw}"));
                }
            }
        }
        Ok(None) // no pattern matched; caller can count as malformed
    }
}