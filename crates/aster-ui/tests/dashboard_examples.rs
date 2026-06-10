// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use aster_ui::{Dashboard, FlexDirection, Widget, WidgetKind};
use std::path::{Path, PathBuf};

fn workspace_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn count_widgets(widget: &Widget) -> usize {
    let children = match widget.kind() {
        WidgetKind::Flex { children, .. } | WidgetKind::Stack { children } => children,
        WidgetKind::Text { .. }
        | WidgetKind::Image { .. }
        | WidgetKind::Spacer
        | WidgetKind::Progress { .. } => return 1,
    };

    1 + children.iter().map(count_widgets).sum::<usize>()
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

    let WidgetKind::Flex { direction, .. } = dashboard.root().kind() else {
        panic!("system overview root should be a flex widget");
    };
    assert_eq!(*direction, FlexDirection::Row);
}

#[test]
fn loads_storage_overview_example() {
    let dashboard = Dashboard::load(workspace_path(
        "examples/dashboards/storage-overview/dashboard.toml",
    ))
    .unwrap();

    assert_eq!(dashboard.root().id(), Some("storage-overview"));
    assert_eq!(count_widgets(dashboard.root()), 11);

    let WidgetKind::Flex { direction, .. } = dashboard.root().kind() else {
        panic!("storage overview root should be a flex widget");
    };
    assert_eq!(*direction, FlexDirection::Column);
}
