// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::binding::Binding;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProgressOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Widget {
    source_path: String,
    id: Option<String>,
    classes: Vec<String>,
    kind: WidgetKind,
}

impl Widget {
    pub(crate) fn new(
        source_path: String,
        id: Option<String>,
        classes: Vec<String>,
        kind: WidgetKind,
    ) -> Self {
        Self {
            source_path,
            id,
            classes,
            kind,
        }
    }

    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn classes(&self) -> &[String] {
        &self.classes
    }

    pub fn kind(&self) -> &WidgetKind {
        &self.kind
    }

    pub fn type_name(&self) -> &'static str {
        match self.kind {
            WidgetKind::Flex {
                direction: FlexDirection::Row,
                ..
            } => "row",
            WidgetKind::Flex {
                direction: FlexDirection::Column,
                ..
            } => "column",
            WidgetKind::Stack { .. } => "stack",
            WidgetKind::Text { .. } => "text",
            WidgetKind::Image { .. } => "image",
            WidgetKind::Spacer => "spacer",
            WidgetKind::Progress { .. } => "progress",
        }
    }

    pub fn children(&self) -> &[Widget] {
        match &self.kind {
            WidgetKind::Flex { children, .. } | WidgetKind::Stack { children } => children,
            WidgetKind::Text { .. }
            | WidgetKind::Image { .. }
            | WidgetKind::Spacer
            | WidgetKind::Progress { .. } => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Flex {
        direction: FlexDirection,
        children: Vec<Widget>,
    },
    Stack {
        children: Vec<Widget>,
    },
    Text {
        text: Binding,
    },
    Image {
        source: PathBuf,
    },
    Spacer,
    Progress {
        value: Binding,
        min: f64,
        max: f64,
        orientation: ProgressOrientation,
    },
}
