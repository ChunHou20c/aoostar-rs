// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use image::GenericImageView;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

fn workspace_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
        .canonicalize()
        .unwrap()
}

#[test]
fn render_once_saves_exactly_one_png_without_a_display() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_asterctl"))
        .current_dir(directory.path())
        .args([
            "--dashboard",
            workspace_path("examples/dashboards/system-overview/dashboard.toml")
                .to_str()
                .unwrap(),
            "--sensor-path",
            workspace_path("examples/dashboards/data/system-values.txt")
                .to_str()
                .unwrap(),
            "--render-once",
            "--save",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let files = fs::read_dir(directory.path().join("out"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(files, vec![directory.path().join("out/dashboard.png")]);
    assert_eq!(
        image::open(&files[0]).unwrap().to_rgba8().dimensions(),
        (960, 376)
    );
}

#[test]
fn render_once_requires_dashboard_and_save() {
    let output = Command::new(env!("CARGO_BIN_EXE_asterctl"))
        .args(["--render-once"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--dashboard"), "{stderr}");
    assert!(stderr.contains("--save"), "{stderr}");
}

#[test]
fn continuous_simulation_saves_an_initial_frame() {
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_asterctl"))
        .current_dir(directory.path())
        .args([
            "--dashboard",
            workspace_path("examples/dashboards/system-overview/dashboard.toml")
                .to_str()
                .unwrap(),
            "--sensor-path",
            workspace_path("examples/dashboards/data/system-values.txt")
                .to_str()
                .unwrap(),
            "--simulate",
            "--save",
        ])
        .spawn()
        .unwrap();
    let output = directory.path().join("out/dashboard-0001.png");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut rendered = false;
    while Instant::now() < deadline {
        rendered = image::open(&output)
            .map(|image| image.dimensions() == (960, 376))
            .unwrap_or(false);
        if rendered {
            break;
        }
        sleep(Duration::from_millis(50));
    }
    child.kill().unwrap();
    let status = child.wait().unwrap();

    assert!(!status.success());
    assert!(rendered);
}
