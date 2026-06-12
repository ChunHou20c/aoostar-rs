// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::sensors::{read_sensor_values, start_file_slurper_with_notifications};
use aster_ui::{Dashboard, Renderer};
use asterctl_lcd::{AooScreen, DISPLAY_SIZE};
use image::RgbaImage;
use log::{debug, info, warn};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
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
    let dashboard_path = fs::canonicalize(dashboard_path)?;
    let dashboard = load_lcd_dashboard(&dashboard_path)?;
    let values = Arc::new(RwLock::new(HashMap::new()));
    let sensor_changes = start_file_slurper_with_notifications(sensor_path, values.clone(), None)?;
    let output_dir = output_dir.map(Path::to_path_buf);
    if let Some(output_dir) = &output_dir {
        fs::create_dir_all(output_dir)?;
    }

    let (invalidations_tx, invalidations_rx) = mpsc::channel();
    let sensor_tx = invalidations_tx.clone();
    std::thread::spawn(move || {
        while sensor_changes.recv().is_ok() {
            if sensor_tx.send(Invalidation::Values).is_err() {
                return;
            }
        }
    });

    let watched_files = Arc::new(RwLock::new(HashSet::new()));
    let reload_tx = invalidations_tx.clone();
    let callback_files = watched_files.clone();
    let mut watcher =
        notify::recommended_watcher(move |event: notify::Result<Event>| match event {
            Ok(event) => {
                debug!("Dashboard asset event {:?}: {:?}", event.kind, event.paths);
                if matches!(
                    event.kind,
                    EventKind::Any
                        | EventKind::Create(_)
                        | EventKind::Modify(_)
                        | EventKind::Remove(_)
                ) && event.paths.iter().any(|path| {
                    let files = callback_files.read().unwrap();
                    files.contains(path)
                        || fs::canonicalize(path)
                            .map(|path| files.contains(&path))
                            .unwrap_or(false)
                }) {
                    let _ = reload_tx.send(Invalidation::Reload);
                }
            }
            Err(error) => warn!("dashboard watch error: {error}"),
        })?;
    let mut watched_directories = HashSet::new();
    update_dashboard_watches(
        &mut watcher,
        &mut watched_directories,
        &watched_files,
        &dashboard,
    )?;

    run_reloading_dashboard(
        dashboard_path,
        dashboard,
        values,
        invalidations_rx,
        DASHBOARD_DEBOUNCE,
        &mut watcher,
        &mut watched_directories,
        watched_files,
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

fn load_lcd_dashboard(path: &Path) -> anyhow::Result<Dashboard> {
    let dashboard = Dashboard::load(path)?;
    anyhow::ensure!(
        (dashboard.options().width(), dashboard.options().height()) == DISPLAY_SIZE,
        "dashboard size must be {}x{} for LCD output, got {}x{}",
        DISPLAY_SIZE.0,
        DISPLAY_SIZE.1,
        dashboard.options().width(),
        dashboard.options().height()
    );
    Ok(dashboard)
}

#[derive(Clone, Copy)]
enum Invalidation {
    Values,
    Reload,
}

#[allow(clippy::too_many_arguments)]
fn run_reloading_dashboard<F>(
    dashboard_path: PathBuf,
    mut dashboard: Dashboard,
    values: Arc<RwLock<HashMap<String, String>>>,
    invalidations: Receiver<Invalidation>,
    debounce: Duration,
    watcher: &mut RecommendedWatcher,
    watched_directories: &mut HashSet<PathBuf>,
    watched_files: Arc<RwLock<HashSet<PathBuf>>>,
    mut handle_frame: F,
) -> anyhow::Result<()>
where
    F: FnMut(&RgbaImage, usize) -> anyhow::Result<()>,
{
    let mut renderer = Renderer::new(&dashboard)?;
    let mut previous_frame = None;
    let mut frame_number = 0;
    let mut reload_error: Option<String> = None;
    let mut render = true;

    loop {
        if render {
            let current_values = values.read().expect("sensor values lock poisoned").clone();
            let started = Instant::now();
            let frame = renderer.render_with_values(&dashboard, &current_values)?;
            debug!("Dashboard rendered in {}ms", started.elapsed().as_millis());
            if previous_frame.as_ref() != Some(&frame) {
                frame_number += 1;
                handle_frame(&frame, frame_number)?;
                previous_frame = Some(frame);
            } else {
                debug!("Dashboard frame unchanged; skipping save and transmission");
            }
        }

        let first = match invalidations.recv() {
            Ok(invalidation) => invalidation,
            Err(_) => return Ok(()),
        };
        let reload = debounce_invalidations(&invalidations, debounce, first);
        render = true;
        if reload {
            let current_values = values.read().expect("sensor values lock poisoned").clone();
            match prepare_reload(&dashboard_path, &current_values) {
                Ok((candidate, candidate_renderer, frame)) => {
                    update_dashboard_watches(
                        watcher,
                        watched_directories,
                        &watched_files,
                        &candidate,
                    )?;
                    dashboard = candidate;
                    renderer = candidate_renderer;
                    reload_error = None;
                    if previous_frame.as_ref() != Some(&frame) {
                        frame_number += 1;
                        handle_frame(&frame, frame_number)?;
                        previous_frame = Some(frame);
                    }
                    info!("Reloaded dashboard configuration");
                    render = false;
                }
                Err(error) => {
                    let message = error.to_string();
                    if reload_error.as_deref() != Some(message.as_str()) {
                        warn!("Failed to reload dashboard; keeping last valid frame: {error:#}");
                        reload_error = Some(message);
                    }
                    render = false;
                }
            }
        }
    }
}

fn prepare_reload(
    dashboard_path: &Path,
    values: &HashMap<String, String>,
) -> anyhow::Result<(Dashboard, Renderer, RgbaImage)> {
    let dashboard = load_lcd_dashboard(dashboard_path)?;
    let mut renderer = Renderer::new(&dashboard)?;
    let frame = renderer.render_with_values(&dashboard, values)?;
    Ok((dashboard, renderer, frame))
}

fn update_dashboard_watches(
    watcher: &mut RecommendedWatcher,
    watched_directories: &mut HashSet<PathBuf>,
    watched_files: &Arc<RwLock<HashSet<PathBuf>>>,
    dashboard: &Dashboard,
) -> notify::Result<()> {
    let files: HashSet<_> = dashboard.asset_paths().into_iter().collect();
    let directories: HashSet<_> = files
        .iter()
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect();

    for directory in watched_directories.difference(&directories) {
        watcher.unwatch(directory)?;
    }
    for directory in directories.difference(watched_directories) {
        watcher.watch(directory, RecursiveMode::NonRecursive)?;
    }
    *watched_directories = directories;
    *watched_files.write().unwrap() = files;
    Ok(())
}

#[cfg(test)]
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

#[cfg(test)]
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

fn debounce_invalidations(
    invalidations: &Receiver<Invalidation>,
    debounce: Duration,
    first: Invalidation,
) -> bool {
    let mut reload = matches!(first, Invalidation::Reload);
    let mut deadline = Instant::now() + debounce;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match invalidations.recv_timeout(remaining) {
            Ok(invalidation) => {
                reload |= matches!(invalidation, Invalidation::Reload);
                deadline = Instant::now() + debounce;
            }
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return reload,
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
