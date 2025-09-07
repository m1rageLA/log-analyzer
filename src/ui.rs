#![cfg(feature = "gui")]

use crate::analyze::{Analyzer, Filters, Granularity};
use crate::parse::DefaultLogParser;
use crate::report::{build_summary, JsonSummary};
use eframe::{egui, App};
use egui::{RichText, ComboBox};
use egui_extras::{Column, TableBuilder};
use rfd::FileDialog;
use std::path::PathBuf;
use egui_plot::{Plot, Line, PlotPoints};

pub fn launch() -> anyhow::Result<()> {
    let native_options = eframe::NativeOptions::default();
    // eframe::Error не Send/Sync → оборачиваем в anyhow через строку
    eframe::run_native(
        "Log Analyzer (GUI)",
        native_options,
        Box::new(|_cc| Ok(Box::<GuiApp>::default())),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}

#[derive(Default)]
struct GuiApp {
    file: Option<PathBuf>,
    keyword: String,
    from: String,
    to: String,
    gran: Granularity,
    summary: Option<JsonSummary>,
    info_text: String,
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading(RichText::new("Log File Analyzer").size(24.0));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // выбор файла
            ui.horizontal(|ui| {
                if ui.button("Open .log...").clicked() {
                    if let Some(p) = FileDialog::new().add_filter("Log", &["log"]).pick_file() {
                        self.file = Some(p);
                        self.info_text.clear();
                        self.summary = None;
                    }
                }
                if let Some(f) = &self.file {
                    ui.label(f.display().to_string());
                }
            });

            ui.separator();

            // фильтры
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.keyword).hint_text("keyword"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.from)
                        .hint_text("from: YYYY-MM-DD HH:MM:SS"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.to).hint_text("to:   YYYY-MM-DD HH:MM:SS"),
                );

                ComboBox::from_label("Granularity")
                    .selected_text(match self.gran {
                        Granularity::Minute => "Minute",
                        Granularity::Hour => "Hour",
                        Granularity::Day => "Day",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.gran, Granularity::Minute, "Minute");
                        ui.selectable_value(&mut self.gran, Granularity::Hour, "Hour");
                        ui.selectable_value(&mut self.gran, Granularity::Day, "Day");
                    });

                if ui.button("Analyze").clicked() {
                    self.run_analysis();
                }
            });

            if !self.info_text.is_empty() {
                ui.label(self.info_text.clone());
            }

            // результаты
            if let Some(sum) = &self.summary {
                ui.separator();
                ui.label(RichText::new("Summary").strong());
                ui.label(format!(
                    "Total: {} | Malformed: {}",
                    sum.total_entries, sum.malformed_lines
                ));
                ui.label(format!(
                    "INFO: {}  WARNING: {}  ERROR: {}",
                    sum.counts.info, sum.counts.warning, sum.counts.error
                ));
                if let Some(f) = &sum.first_log {
                    ui.label(format!("First: {}", f));
                }
                if let Some(l) = &sum.last_log {
                    ui.label(format!("Last:  {}", l));
                }

                ui.add_space(8.0);
                ui.label(RichText::new("Top Errors").strong());
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto())
                    .column(Column::remainder())
                    .body(|mut body| {
                        for (msg, n) in &sum.common_errors {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(format!("{n}"));
                                });
                                row.col(|ui| {
                                    ui.label(msg);
                                });
                            });
                        }
                    });

                ui.add_space(12.0);
                ui.label(RichText::new("Timeline").strong());

                let points: Vec<[f64; 2]> = sum
                    .timeline
                    .iter()
                    .enumerate()
                    .map(|(i, (_, v))| [i as f64, *v as f64])
                    .collect();

                let plot = Plot::new("timeline").view_aspect(3.0);
                plot.show(ui, |pui| {
                    let line = Line::new(PlotPoints::from(points));
                    pui.line(line);
                });
            }
        });
    }
}

impl GuiApp {
    fn run_analysis(&mut self) {
        if self.file.is_none() {
            self.info_text = "Select a .log file first".into();
            return;
        }
        let filters = match Filters::from_cli(
            Some(self.keyword.as_str()).filter(|s| !s.is_empty()),
            if self.from.is_empty() {
                None
            } else {
                Some(self.from.as_str())
            },
            if self.to.is_empty() {
                None
            } else {
                Some(self.to.as_str())
            },
            None,
        ) {
            Ok(f) => f,
            Err(e) => {
                self.info_text = format!("Filter error: {e}");
                return;
            }
        };
        let mut parser = DefaultLogParser::new();
        let mut analyzer = Analyzer::new(self.gran);
        if let Err(e) = analyzer.consume_file(&mut parser, self.file.as_ref().unwrap()) {
            self.info_text = format!("Read error: {e}");
            return;
        }
        self.summary = Some(build_summary(&analyzer, &filters));
    }
}

// Assuming Granularity is defined in another module, implement PartialEq for it.
impl PartialEq for Granularity {
    fn eq(&self, other: &Self) -> bool {
        // Add logic to compare variants of Granularity
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}
