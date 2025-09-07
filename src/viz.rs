use crate::analyze::Granularity;
use crate::report::JsonSummary;
use anyhow::Result;
use plotters::prelude::*;

pub fn save_level_barchart(path: &std::path::Path, summary: &JsonSummary) -> Result<()> {
    let root = BitMapBackend::new(path, (900, 500)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_y = [summary.counts.info, summary.counts.warning, summary.counts.error]
        .into_iter()
        .max()
        .unwrap_or(1);

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Logs per Level", ("sans-serif", 28))
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0i32..3i32, 0u64..(max_y + max_y / 5 + 1))?;

    chart
        .configure_mesh()
        .x_labels(3)
        .x_label_formatter(&|x| match x {
            0 => "INFO".into(),
            1 => "WARNING".into(),
            2 => "ERROR".into(),
            _ => "".into(),
        })
        .y_desc("Count")
        .draw()?;

    let bars = vec![summary.counts.info, summary.counts.warning, summary.counts.error];
    for (i, val) in bars.iter().enumerate() {
        let x0 = i as i32;
        let x1 = (i + 1) as i32;
        chart.draw_series(std::iter::once(Rectangle::new(
            [(x0, 0u64), (x1, *val)],
            RGBColor(50 + 60 * i as u8, 100, 200).filled(),
        )))?;
    }

    root.present()?;
    Ok(())
}

pub fn save_timeline_chart(path: &std::path::Path, summary: &JsonSummary, _gran: Granularity) -> Result<()> {
    let root = BitMapBackend::new(path, (1200, 500)).into_drawing_area();
    root.fill(&WHITE)?;

    // X as index (uniform spacing), label with first/last
    let n = summary.timeline.len().max(1);
    let max_y = summary.timeline.iter().map(|(_, v)| *v).max().unwrap_or(1);

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Log Frequency Over Time", ("sans-serif", 28))
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0i32..(n as i32 - 1).max(0), 0u64..(max_y + max_y / 5 + 1))?;

    chart
        .configure_mesh()
        .y_desc("Entries")
        .x_desc("Bucket")
        .x_labels(10)
        .x_label_formatter(&|i| {
            if summary.timeline.is_empty() {
                return "".into();
            }
            let idx = (*i as usize).min(summary.timeline.len() - 1);
            summary.timeline[idx].0.clone()
        })
        .draw()?;

    let series: Vec<(i32, u64)> = summary
        .timeline
        .iter()
        .enumerate()
        .map(|(i, (_, v))| (i as i32, *v))
        .collect();
    chart.draw_series(LineSeries::new(series.into_iter(), &BLUE))?;

    root.present()?;
    Ok(())
}