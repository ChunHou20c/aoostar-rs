// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::sensors::{read_sensor_values, start_file_slurper_with_notifications};
use aster_ui::{Dashboard, Renderer};
use asterctl_lcd::{AooScreen, DISPLAY_SIZE};
use image::RgbaImage;
use log::{debug, info};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

const DASHBOARD_DEBOUNCE: Duration = Duration::from_millis(30);

pub fn render_dashboard_once(
    dashboard_path: impl AsRef<Path>,
    sensor_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<PathBuf> {
    let dashboard = Dashboard::load(dashboard_path)?;
    let values = read_sensor_values(sensor_path, None)?;
    let image = Renderer::new(&dashboard)?.render_with_values(&dashboard, &values)?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;
    let output_path = output_dir.join("dashboard.png");
    image.save(&output_path)?;
    Ok(output_path)
}

pub fn run_dashboard(
    screen: &mut AooScreen,
    dashboard_path: impl AsRef<Path>,
    sensor_path: impl Into<PathBuf>,
    output_dir: Option<&Path>,
) -> anyhow::Result<()> {
    let dashboard = Dashboard::load(dashboard_path)?;
    anyhow::ensure!(
        (dashboard.options().width(), dashboard.options().height()) == DISPLAY_SIZE,
        "dashboard size must be {}x{} for LCD output, got {}x{}",
        DISPLAY_SIZE.0,
        DISPLAY_SIZE.1,
        dashboard.options().width(),
        dashboard.options().height()
    );
    let values = Arc::new(RwLock::new(HashMap::new()));
    let changes = start_file_slurper_with_notifications(sensor_path, values.clone(), None)?;
    let output_dir = output_dir.map(Path::to_path_buf);
    if let Some(output_dir) = &output_dir {
        fs::create_dir_all(output_dir)?;
    }

    run_dashboard_updates(
        &dashboard,
        values,
        changes,
        DASHBOARD_DEBOUNCE,
        |image, frame_number| {
            if let Some(output_dir) = &output_dir {
                let output = output_dir.join(format!("dashboard-{frame_number:04}.png"));
                image.save(&output)?;
                info!("Saved dashboard frame to {output:?}");
            }
            screen.send_image(image)?;
            Ok(())
        },
    )
}

fn run_dashboard_updates<F>(
    dashboard: &Dashboard,
    values: Arc<RwLock<HashMap<String, String>>>,
    changes: Receiver<()>,
    debounce: Duration,
    mut handle_frame: F,
) -> anyhow::Result<()>
where
    F: FnMut(&RgbaImage, usize) -> anyhow::Result<()>,
{
    let mut renderer = Renderer::new(dashboard)?;
    let mut previous_frame = None;
    let mut frame_number = 0;

    loop {
        let current_values = values.read().expect("sensor values lock poisoned").clone();
        let started = Instant::now();
        let frame = renderer.render_with_values(dashboard, &current_values)?;
        debug!("Dashboard rendered in {}ms", started.elapsed().as_millis());
        if previous_frame.as_ref() != Some(&frame) {
            frame_number += 1;
            handle_frame(&frame, frame_number)?;
            previous_frame = Some(frame);
        } else {
            debug!("Dashboard frame unchanged; skipping save and transmission");
        }

        match changes.recv() {
            Ok(()) => debounce_changes(&changes, debounce),
            Err(_) => return Ok(()),
        }
    }
}

fn debounce_changes(changes: &Receiver<()>, debounce: Duration) {
    let mut deadline = Instant::now() + debounce;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match changes.recv_timeout(remaining) {
            Ok(()) => deadline = Instant::now() + debounce,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn skips_unchanged_frames_and_debounces_updates() {
        let dashboard = Dashboard::load(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/dashboards/system-overview/dashboard.toml"),
        )
        .unwrap();
        let values = Arc::new(RwLock::new(HashMap::from([
            ("cpu_percent".to_string(), "10".to_string()),
            ("cpu_temperature".to_string(), "20".to_string()),
            ("memory_usage".to_string(), "30".to_string()),
        ])));
        let (tx, rx) = mpsc::channel();
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        drop(tx);
        let mut handled = 0;

        run_dashboard_updates(&dashboard, values, rx, Duration::from_millis(1), |_, _| {
            handled += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(handled, 1);
    }

    #[test]
    fn a_burst_of_changes_produces_one_new_frame() {
        let dashboard = Dashboard::load(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/dashboards/system-overview/dashboard.toml"),
        )
        .unwrap();
        let values = Arc::new(RwLock::new(HashMap::from([
            ("cpu_percent".to_string(), "10".to_string()),
            ("cpu_temperature".to_string(), "20".to_string()),
            ("memory_usage".to_string(), "30".to_string()),
        ])));
        let update_values = values.clone();
        let (tx, rx) = mpsc::channel();
        let mut sender = Some(tx);
        let mut frame_numbers = Vec::new();

        run_dashboard_updates(
            &dashboard,
            values,
            rx,
            Duration::from_millis(1),
            |_, frame_number| {
                frame_numbers.push(frame_number);
                if frame_number == 1 {
                    update_values
                        .write()
                        .unwrap()
                        .insert("cpu_percent".to_string(), "80".to_string());
                    let tx = sender.take().unwrap();
                    tx.send(()).unwrap();
                    tx.send(()).unwrap();
                }
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(frame_numbers, vec![1, 2]);
    }
}
