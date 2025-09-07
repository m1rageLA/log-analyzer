mod model;
mod parse;
mod analyze;
mod report;
mod viz;
#[cfg(feature = "gui")] mod ui;

use clap::{Parser, ValueEnum};
use anyhow::{Context, Result};
use std::path::PathBuf;
use walkdir::WalkDir;
use crate::parse::DefaultLogParser;
use crate::analyze::{Analyzer, Filters, Granularity};
use crate::model::Level; // Level lives in model
use crate::report::build_summary;
use crate::viz::{save_level_barchart, save_timeline_chart};

/// Log File Analyzer (Rust)
/// Parses .log files, filters, summarizes, exports JSON, and renders charts. Can also run a GUI.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// One or more files or directories (recursively scanned for *.log)
    #[arg(required_unless_present = "gui")]
    inputs: Vec<PathBuf>,

    /// Keyword filter (case-insensitive substring)
    #[arg(long)]
    keyword: Option<String>,

    /// Minimum timestamp (inclusive). Examples: "2025-09-05 14:00:00", "2025-09-05T14:00:00"
    #[arg(long, value_name = "DATETIME")]
    from: Option<String>,

    /// Maximum timestamp (exclusive)
    #[arg(long, value_name = "DATETIME")]
    to: Option<String>,

    /// Only include this level
    #[arg(long, value_enum)]
    level: Option<LevelArg>,

    /// Timeline granularity
    #[arg(long, default_value_t = GranularityArg::Hour, value_enum)]
    granularity: GranularityArg,

    /// Save summary JSON to this path
    #[arg(long)]
    json_out: Option<PathBuf>,

    /// Save a bar chart (PNG) with counts per level
    #[arg(long)]
    bar_out: Option<PathBuf>,

    /// Save a timeline chart (PNG) of log frequency
    #[arg(long)]
    timeline_out: Option<PathBuf>,

    /// Launch GUI instead of CLI
    #[arg(long)]
    gui: bool,
}

#[derive(Clone, Debug, ValueEnum)]
enum LevelArg { Info, Warning, Error }
impl From<LevelArg> for Level {
    fn from(v: LevelArg) -> Self {
        match v { LevelArg::Info => Level::Info, LevelArg::Warning => Level::Warning, LevelArg::Error => Level::Error }
    }
}

#[derive(Clone, Debug, ValueEnum)]
enum GranularityArg { Minute, Hour, Day }
impl From<GranularityArg> for Granularity {
    fn from(v: GranularityArg) -> Self {
        match v { GranularityArg::Minute => Granularity::Minute, GranularityArg::Hour => Granularity::Hour, GranularityArg::Day => Granularity::Day }
    }
}

fn gather_log_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_file() {
            out.push(p.clone());
        } else if p.is_dir() {
            for entry in WalkDir::new(p).into_iter().flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() { if ext == "log" { out.push(path.to_path_buf()); } }
                }
            }
        }
    }
    out
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.gui {
        #[cfg(feature = "gui")]
        { return ui::launch(); }
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("This build has GUI disabled. Rebuild with `--features gui`.");
            std::process::exit(2);
        }
    }

    let files = gather_log_files(&cli.inputs);
    if files.is_empty() { anyhow::bail!("No .log files found in provided inputs"); }

    let mut parser = DefaultLogParser::new();

    let filters = Filters::from_cli(
        cli.keyword.as_deref(),
        cli.from.as_deref(),
        cli.to.as_deref(),
        cli.level.map(Into::into),
    )?;

    let gran: Granularity = cli.granularity.into();
    let mut analyzer = Analyzer::new(gran);

    for path in files {
        analyzer
            .consume_file(&mut parser, &path)
            .with_context(|| format!("Failed reading {path:?}"))?;
    }

    let summary = build_summary(&analyzer, &filters);

    // Print human summary
    println!("====== SUMMARY ======");
    println!("Total entries (filtered): {}", summary.total_entries);
    println!(
        "Counts: INFO={}, WARNING={}, ERROR={}",
        summary.counts.info, summary.counts.warning, summary.counts.error
    );
    println!("Malformed lines: {}", summary.malformed_lines);
    if let Some(first) = summary.first_log.as_deref() { println!("First log: {first}"); }
    if let Some(last) = summary.last_log.as_deref() { println!("Last log:  {last}"); }
    if !summary.common_errors.is_empty() {
        println!("
Top error messages:");
        for (msg, n) in &summary.common_errors { println!("  {n:>6}  {msg}"); }
    }

    // Save JSON
    if let Some(path) = cli.json_out.as_deref() {
        std::fs::write(path, serde_json::to_vec_pretty(&summary)?)?;
        println!("Saved JSON -> {}", path.display());
    }

    // Charts
    if let Some(path) = cli.bar_out.as_deref() {
        save_level_barchart(path, &summary)
            .with_context(|| format!("Saving bar chart to {}", path.display()))?;
        println!("Saved bar chart -> {}", path.display());
    }
    if let Some(path) = cli.timeline_out.as_deref() {
        save_timeline_chart(path, &summary, gran)
            .with_context(|| format!("Saving timeline to {}", path.display()))?;
        println!("Saved timeline -> {}", path.display());
    }

    Ok(())
}