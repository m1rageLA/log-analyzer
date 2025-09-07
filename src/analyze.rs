use crate::model::{LogEntry, Level};
use crate::parse::LogParser;
use chrono::{NaiveDateTime, Datelike, Timelike, NaiveDate};
use std::collections::{BTreeMap, HashMap};
use std::fs::File; use std::io::{BufRead, BufReader};
use anyhow::Result;

#[derive(Clone, Copy, Debug)]
pub enum Granularity { Minute, Hour, Day }

impl Default for Granularity {
    fn default() -> Self { Granularity::Hour }
}
#[derive(Clone, Debug)]
pub struct Filters {
    pub keyword: Option<String>,
    pub from: Option<NaiveDateTime>,
    pub to: Option<NaiveDateTime>,
    pub level: Option<Level>,
}

impl Filters {
    pub fn from_cli(keyword: Option<&str>, from: Option<&str>, to: Option<&str>, level: Option<Level>) -> anyhow::Result<Self> {
        use chrono::NaiveDateTime;
        fn parse_opt(s: Option<&str>) -> anyhow::Result<Option<NaiveDateTime>> {
            if let Some(val) = s {
                let tried = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d"];
                for f in tried { if let Ok(dt) = NaiveDateTime::parse_from_str(val, f) { return Ok(Some(dt)); } }
                // If only date was given, set midnight
                if let Ok(date) = chrono::NaiveDate::parse_from_str(val, "%Y-%m-%d") { return Ok(Some(date.and_hms_opt(0,0,0).unwrap())); }
                anyhow::bail!("Could not parse datetime: {val}");
            }
            Ok(None)
        }
        Ok(Self { keyword: keyword.map(|s| s.to_lowercase()), from: parse_opt(from)?, to: parse_opt(to)?, level })
    }

    fn pass(&self, e: &LogEntry) -> bool {
        if let Some(lv) = self.level { if e.level != lv { return false; } }
        if let Some(f) = self.from { if e.ts < f { return false; } }
        if let Some(t) = self.to { if e.ts >= t { return false; } }
        if let Some(k) = &self.keyword { if !e.message.to_lowercase().contains(k) { return false; } }
        true
    }
}

#[derive(Default)]
pub struct Analyzer {
    pub granularity: Granularity,
    // Global counts
    pub info: u64,
    pub warning: u64,
    pub error: u64,
    pub malformed_lines: u64,

    // First/last timestamps
    pub first: Option<NaiveDateTime>,
    pub last: Option<NaiveDateTime>,

    // Timeline counts (filtered later)
    pub timeline: BTreeMap<NaiveDateTime, u64>,

    // Common error messages
    pub error_messages: HashMap<String, u64>,
}

impl Analyzer {
    pub fn new(granularity: Granularity) -> Self { Self { granularity, ..Default::default() } }

    fn bucket(&self, ts: NaiveDateTime) -> NaiveDateTime {
        match self.granularity {
            Granularity::Minute => ts.with_second(0).unwrap().with_nanosecond(0).unwrap(),
            Granularity::Hour => ts.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap(),
            Granularity::Day => NaiveDate::from_ymd_opt(ts.year(), ts.month(), ts.day()).unwrap().and_hms_opt(0,0,0).unwrap(),
        }
    }

    pub fn consume_file<P: LogParser>(&mut self, parser: &mut P, path: &std::path::Path) -> Result<()> {
        let f = File::open(path)?; let r = BufReader::new(f);
        for line in r.lines() {
            let line = line?;
            match parser.parse_line(&line) {
                Ok(Some(entry)) => { self.consume_entry(entry); },
                Ok(None) => { self.malformed_lines += 1; },
                Err(_) => { self.malformed_lines += 1; },
            }
        }
        Ok(())
    }

    fn consume_entry(&mut self, e: LogEntry) {
        match e.level { Level::Info => self.info += 1, Level::Warning => self.warning += 1, Level::Error => self.error += 1 }
        self.first = Some(self.first.map_or(e.ts, |cur| cur.min(e.ts)));
        self.last = Some(self.last.map_or(e.ts, |cur| cur.max(e.ts)));
        let b = self.bucket(e.ts);
        *self.timeline.entry(b).or_default() += 1;
        if matches!(e.level, Level::Error) {
            *self.error_messages.entry(e.message).or_default() += 1;
        }
    }
}