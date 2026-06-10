// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use aster_ui::{Dashboard, Renderer, ValueMap};
use image::Rgba;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn font_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fonts/DejaVuSans.ttf")
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn bindings_change_text_layout_and_apply_filters() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
.label { font-family: "DejaVu Sans"; font-size: 20px; color: #ffffff; }
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        format!(
            r#"
[dashboard]
width = 240
height = 40
stylesheet = "dashboard.css"
fonts = [{}]

[root]
type = "row"

[[root.children]]
type = "text"
class = ["label"]
text = 'CPU {{{{ cpu | number(1) }}}}% {{{{ host | default("unknown") }}}}'
"#,
            toml::Value::String(font_path())
        ),
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let mut renderer = Renderer::new(&dashboard).unwrap();
    let missing_layout = renderer
        .compute_layout_with_values(&dashboard, &ValueMap::new())
        .unwrap();
    let missing_width = missing_layout
        .root()
        .find("root.children[0]")
        .unwrap()
        .width();

    let values = ValueMap::from([
        ("cpu".to_string(), "47.66".to_string()),
        ("host".to_string(), "aoostar".to_string()),
    ]);
    let resolved_layout = renderer
        .compute_layout_with_values(&dashboard, &values)
        .unwrap();
    let resolved_width = resolved_layout
        .root()
        .find("root.children[0]")
        .unwrap()
        .width();

    assert!(resolved_width > missing_width);
}

#[test]
fn progress_renders_horizontal_and_vertical_values() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
progress {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    background-color: #112233;
    color: #00ff00;
}
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 40
height = 20
stylesheet = "dashboard.css"

[root]
type = "row"

[[root.children]]
type = "progress"
value = "{{ horizontal }}"

[[root.children]]
type = "progress"
value = "{{ vertical }}"
orientation = "vertical"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let values = ValueMap::from([
        ("horizontal".to_string(), "50".to_string()),
        ("vertical".to_string(), "25".to_string()),
    ]);
    let output = Renderer::new(&dashboard)
        .unwrap()
        .render_with_values(&dashboard, &values)
        .unwrap();

    let track = Rgba([17, 34, 51, 255]);
    let fill = Rgba([0, 255, 0, 255]);
    assert_eq!(output.get_pixel(5, 10), &fill);
    assert_eq!(output.get_pixel(15, 10), &track);
    assert_eq!(output.get_pixel(30, 2), &track);
    assert_eq!(output.get_pixel(30, 18), &fill);
}

#[test]
fn progress_clamps_values_and_only_changes_its_fill_region() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        "progress { width: 20px; height: 10px; color: #ffffff; }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 20
height = 10
stylesheet = "dashboard.css"

[root]
type = "progress"
value = "{{ value }}"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let mut renderer = Renderer::new(&dashboard).unwrap();
    let empty = renderer
        .render_with_values(&dashboard, &ValueMap::new())
        .unwrap();
    let full = renderer
        .render_with_values(
            &dashboard,
            &ValueMap::from([("value".to_string(), "150".to_string())]),
        )
        .unwrap();

    assert!(empty.pixels().all(|pixel| pixel.0 == [0, 0, 0, 0]));
    assert!(full.pixels().all(|pixel| pixel.0 == [255, 255, 255, 255]));
}

#[test]
fn malformed_numeric_values_report_the_widget_path() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        "progress { width: 20px; height: 10px; }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 20
height = 10
stylesheet = "dashboard.css"

[root]
type = "progress"
value = "{{ value }}"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let error = Renderer::new(&dashboard)
        .unwrap()
        .render_with_values(
            &dashboard,
            &ValueMap::from([("value".to_string(), "not-a-number".to_string())]),
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("at root"), "{error}");
    assert!(error.contains("must be numeric"), "{error}");
}

#[test]
fn invalid_bindings_are_rejected_during_dashboard_load() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("dashboard.css"), "text {}\n").unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 20
height = 10
stylesheet = "dashboard.css"

[root]
type = "text"
text = "{{ value | number(-1) }}"
"#,
    )
    .unwrap();

    let error = Dashboard::load(directory.path().join("dashboard.toml"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("invalid text binding"), "{error}");
}
