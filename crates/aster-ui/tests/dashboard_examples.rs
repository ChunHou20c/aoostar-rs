// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use aster_ui::{Dashboard, FlexDirection, Renderer, ValueMap, Widget, WidgetKind};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn count_widgets(widget: &Widget) -> usize {
    let children = match widget.kind() {
        WidgetKind::Flex { children, .. }
        | WidgetKind::Stack { children }
        | WidgetKind::Conditional { children, .. } => children,
        WidgetKind::Text { .. }
        | WidgetKind::Image { .. }
        | WidgetKind::Spacer
        | WidgetKind::Progress { .. }
        | WidgetKind::CircularProgress { .. }
        | WidgetKind::Graph { .. }
        | WidgetKind::Gauge { .. } => return 1,
    };

    1 + children.iter().map(count_widgets).sum::<usize>()
}

fn load_values(path: impl AsRef<Path>) -> ValueMap {
    fs::read_to_string(workspace_path(path))
        .unwrap()
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

#[test]
fn loads_system_overview_example() {
    let dashboard = Dashboard::load(workspace_path(
        "examples/dashboards/system-overview/dashboard.toml",
    ))
    .unwrap();

    assert_eq!(dashboard.options().width(), 960);
    assert_eq!(dashboard.options().height(), 376);
    assert_eq!(dashboard.root().id(), Some("system-overview"));
    assert_eq!(count_widgets(dashboard.root()), 13);
    let layout = dashboard.compute_layout().unwrap();
    assert_eq!(
        (layout.root().width(), layout.root().height()),
        (960.0, 376.0)
    );

    let WidgetKind::Flex { direction, .. } = dashboard.root().kind() else {
        panic!("system overview root should be a flex widget");
    };
    assert_eq!(*direction, FlexDirection::Row);

    let image = Renderer::new(&dashboard)
        .unwrap()
        .render_with_values(
            &dashboard,
            &load_values("examples/dashboards/data/system-values.txt"),
        )
        .unwrap();
    assert_eq!(image.dimensions(), (960, 376));
}

#[test]
fn loads_storage_overview_example() {
    let dashboard = Dashboard::load(workspace_path(
        "examples/dashboards/storage-overview/dashboard.toml",
    ))
    .unwrap();

    assert_eq!(dashboard.root().id(), Some("storage-overview"));
    assert_eq!(count_widgets(dashboard.root()), 11);
    let layout = dashboard.compute_layout().unwrap();
    assert_eq!(
        (layout.root().width(), layout.root().height()),
        (960.0, 376.0)
    );

    let WidgetKind::Flex { direction, .. } = dashboard.root().kind() else {
        panic!("storage overview root should be a flex widget");
    };
    assert_eq!(*direction, FlexDirection::Column);

    let image = Renderer::new(&dashboard)
        .unwrap()
        .render_with_values(
            &dashboard,
            &load_values("examples/dashboards/data/storage-values.txt"),
        )
        .unwrap();
    assert_eq!(image.dimensions(), (960, 376));
}

#[test]
fn loads_advanced_components_example() {
    let dashboard = Dashboard::load(workspace_path(
        "examples/dashboards/advanced-components/dashboard.toml",
    ))
    .unwrap();

    assert_eq!(dashboard.root().id(), Some("advanced-components"));
    assert_eq!(count_widgets(dashboard.root()), 9);
    let image = Renderer::new(&dashboard)
        .unwrap()
        .render_with_values(
            &dashboard,
            &load_values("examples/dashboards/advanced-components/values.txt"),
        )
        .unwrap();
    assert_eq!(image.dimensions(), (960, 376));
    assert!(image.pixels().any(|pixel| pixel[3] > 0));
}
