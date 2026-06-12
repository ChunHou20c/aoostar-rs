// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::sensors::read_sensor_values;
use aster_ui::{Dashboard, Renderer};
use std::fs;
use std::path::{Path, PathBuf};

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
