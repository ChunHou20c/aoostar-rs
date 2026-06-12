// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use aster_ui::{Dashboard, Renderer, ValueMap, WidgetKind};
use std::fs;
use tempfile::tempdir;

#[test]
fn reusable_components_expand_and_apply_instance_identity() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        ".card { width: 40px; height: 20px; }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 80
height = 20
stylesheet = "dashboard.css"

[components.card]
type = "column"
class = ["card"]

[[components.card.children]]
type = "text"
text = "metric"

[root]
type = "row"

[[root.children]]
type = "component"
component = "card"
id = "first"

[[root.children]]
type = "component"
component = "card"
id = "second"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    assert_eq!(dashboard.root().children().len(), 2);
    assert_eq!(dashboard.root().children()[0].id(), Some("first"));
    assert_eq!(dashboard.root().children()[1].id(), Some("second"));
    assert_eq!(dashboard.root().children()[0].classes(), &["card"]);
    assert!(matches!(
        dashboard.root().children()[0].kind(),
        WidgetKind::Flex { .. }
    ));
}

#[test]
fn reusable_component_cycles_are_rejected() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("dashboard.css"), "row {}\n").unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 80
height = 20
stylesheet = "dashboard.css"

[components.first]
type = "component"
component = "second"

[components.second]
type = "component"
component = "first"

[root]
type = "component"
component = "first"
"#,
    )
    .unwrap();

    let error = Dashboard::load(directory.path().join("dashboard.toml"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("component cycle detected"), "{error}");
    assert!(error.contains("first -> second -> first"), "{error}");
}

#[test]
fn conditional_widgets_collapse_without_resolving_hidden_children() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        "conditional { width: 30px; height: 20px; }\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 60
height = 20
stylesheet = "dashboard.css"

[root]
type = "row"

[[root.children]]
type = "conditional"
value = "{{ enabled }}"
equals = "yes"

[[root.children.children]]
type = "progress"
value = "{{ invalid }}"

[[root.children]]
type = "spacer"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let mut renderer = Renderer::new(&dashboard).unwrap();
    let hidden = renderer
        .compute_layout_with_values(&dashboard, &ValueMap::new())
        .unwrap();
    assert_eq!(hidden.root().find("root.children[0]").unwrap().width(), 0.0);

    let visible = renderer
        .compute_layout_with_values(
            &dashboard,
            &ValueMap::from([("enabled".to_string(), "yes".to_string())]),
        )
        .unwrap();
    assert_eq!(
        visible.root().find("root.children[0]").unwrap().width(),
        30.0
    );
}

#[test]
fn circular_progress_graph_and_gauge_render_sensor_values() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
circular-progress {
    width: 40px;
    height: 40px;
    color: #00ff00;
    border-color: #223344;
}
graph {
    width: 40px;
    height: 40px;
    color: #ff0000;
}
gauge {
    width: 40px;
    height: 40px;
    color: #0000ff;
    border-color: #223344;
}
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 120
height = 40
stylesheet = "dashboard.css"

[root]
type = "row"

[[root.children]]
type = "circular-progress"
value = "{{ circle }}"
thickness = 4

[[root.children]]
type = "graph"
value = "{{ history }}"
min = 0
max = 100
fill = true

[[root.children]]
type = "gauge"
value = "{{ gauge }}"
thickness = 4
needle-width = 2
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let values = ValueMap::from([
        ("circle".to_string(), "75".to_string()),
        ("history".to_string(), "0, 25, 100, 50".to_string()),
        ("gauge".to_string(), "50".to_string()),
    ]);
    let output = Renderer::new(&dashboard)
        .unwrap()
        .render_with_values(&dashboard, &values)
        .unwrap();

    assert!(
        output
            .pixels()
            .any(|pixel| pixel[1] > pixel[0] && pixel[1] > pixel[2])
    );
    assert!(
        output
            .pixels()
            .any(|pixel| pixel[0] > pixel[1] && pixel[0] > pixel[2])
    );
    assert!(
        output
            .pixels()
            .any(|pixel| pixel[2] > pixel[0] && pixel[2] > pixel[1])
    );
}

#[test]
fn malformed_graph_samples_report_the_widget_path() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        "graph { width: 20px; height: 10px; }\n",
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
type = "graph"
value = "{{ history }}"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let error = Renderer::new(&dashboard)
        .unwrap()
        .render_with_values(
            &dashboard,
            &ValueMap::from([("history".to_string(), "1, nope, 3".to_string())]),
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("at root"), "{error}");
    assert!(error.contains("graph samples must be numeric"), "{error}");
}
