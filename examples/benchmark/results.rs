use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;

use bevy::prelude::*;
use bevy_kana::ToF64;
use bevy_kana::ToU32;
use bevy_kana::ToUsize;

use crate::constants::MILLISECONDS_PER_SECOND;

#[derive(Clone)]
pub(super) struct ScenarioResult {
    pub(super) name:          String,
    pub(super) frames:        u32,
    pub(super) average:       f64,
    pub(super) median:        f64,
    pub(super) percentile_95: f64,
    pub(super) percentile_99: f64,
    pub(super) min:           f64,
    pub(super) max:           f64,
}

impl ScenarioResult {
    pub(super) fn average_frames_per_second(&self) -> f64 {
        if self.average > 0.0 {
            MILLISECONDS_PER_SECOND / self.average
        } else {
            0.0
        }
    }
}

pub(super) fn compute_statistics(name: &str, frame_times: &mut [f64]) -> ScenarioResult {
    frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = frame_times.len();
    let sum: f64 = frame_times.iter().sum();
    let average = sum / len.to_f64();
    let median = percentile(frame_times, 50.0);
    let percentile_95 = percentile(frame_times, 95.0);
    let percentile_99 = percentile(frame_times, 99.0);
    let min = frame_times.first().copied().unwrap_or(0.0);
    let max = frame_times.last().copied().unwrap_or(0.0);

    ScenarioResult {
        name: (*name).to_string(),
        frames: len.to_u32(),
        average,
        median,
        percentile_95,
        percentile_99,
        min,
        max,
    }
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let len_f64 = (sorted.len() - 1).to_f64();
    let idx = (pct / 100.0 * len_f64).round().to_u32().to_usize();
    sorted[idx.min(sorted.len() - 1)]
}

pub(super) fn write_results(results: &[ScenarioResult]) {
    let mut table = String::new();
    let _ = writeln!(table, "\n=== bevy_liminal Benchmark Results ===\n");
    let _ = writeln!(
        table,
        "{:<18}| {:>6} | {:>11} | {:>11} | {:>11} | {:>11} | {:>11} | {:>11} | {:>6}",
        "Scenario",
        "Frames",
        "Average(ms)",
        "Median(ms)",
        "95th(ms)",
        "99th(ms)",
        "Min(ms)",
        "Max(ms)",
        "FPS"
    );
    let _ = writeln!(
        table,
        "{:-<18}|{:->8}|{:->13}|{:->13}|{:->13}|{:->13}|{:->13}|{:->13}|{:->8}",
        "", "", "", "", "", "", "", "", ""
    );

    for result in results {
        let _ = writeln!(
            table,
            "{:<18}| {:>6} | {:>11.2} | {:>11.2} | {:>11.2} | {:>11.2} | {:>11.2} | {:>11.2} | {:>6.0}",
            result.name,
            result.frames,
            result.average,
            result.median,
            result.percentile_95,
            result.percentile_99,
            result.min,
            result.max,
            result.average_frames_per_second()
        );
    }

    info!("{table}");

    match write_csv(results) {
        Ok(path) => info!("CSV written to: {path}"),
        Err(error) => warn!("Failed to write CSV: {error}"),
    }
}

fn format_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let output = std::process::Command::new("date")
        .args(["-r", &now.to_string(), "+%Y_%m_%d_%H_%M"])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => format!("{now}"),
    }
}

fn write_csv(results: &[ScenarioResult]) -> Result<String, std::io::Error> {
    let results_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("results");
    std::fs::create_dir_all(&results_dir)?;

    let timestamp = format_timestamp();
    let path = results_dir.join(format!("benchmark_{timestamp}.csv"));
    let mut file = File::create(&path)?;
    writeln!(
        file,
        "scenario,frames,average_ms,median_ms,percentile_95_ms,percentile_99_ms,min_ms,max_ms,average_frames_per_second"
    )?;
    for result in results {
        writeln!(
            file,
            "{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0}",
            result.name,
            result.frames,
            result.average,
            result.median,
            result.percentile_95,
            result.percentile_99,
            result.min,
            result.max,
            result.average_frames_per_second()
        )?;
    }
    Ok(path.display().to_string())
}
