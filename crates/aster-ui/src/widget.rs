// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::binding::Binding;
use crate::{BindingResolveError, ValueMap};
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
pub struct Condition {
    value: Binding,
    comparison: ConditionComparison,
}

impl Condition {
    pub(crate) fn new(value: Binding, comparison: ConditionComparison) -> Self {
        Self { value, comparison }
    }

    pub fn evaluate(&self, values: &ValueMap) -> Result<bool, BindingResolveError> {
        let value = self.value.resolve(values)?;
        Ok(match &self.comparison {
            ConditionComparison::Truthy => {
                let value = value.trim();
                !value.is_empty()
                    && !matches!(
                        value.to_ascii_lowercase().as_str(),
                        "0" | "false" | "no" | "off"
                    )
            }
            ConditionComparison::Equals(expected) => value == *expected,
            ConditionComparison::NotEquals(expected) => value != *expected,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionComparison {
    Truthy,
    Equals(String),
    NotEquals(String),
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
            WidgetKind::CircularProgress { .. } => "circular-progress",
            WidgetKind::Graph { .. } => "graph",
            WidgetKind::Gauge { .. } => "gauge",
            WidgetKind::Conditional { .. } => "conditional",
        }
    }

    pub fn children(&self) -> &[Widget] {
        match &self.kind {
            WidgetKind::Flex { children, .. }
            | WidgetKind::Stack { children }
            | WidgetKind::Conditional { children, .. } => children,
            WidgetKind::Text { .. }
            | WidgetKind::Image { .. }
            | WidgetKind::Spacer
            | WidgetKind::Progress { .. }
            | WidgetKind::CircularProgress { .. }
            | WidgetKind::Graph { .. }
            | WidgetKind::Gauge { .. } => &[],
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
    CircularProgress {
        value: Binding,
        min: f64,
        max: f64,
        start_angle: f32,
        sweep_angle: f32,
        thickness: f32,
    },
    Graph {
        values: Binding,
        min: Option<f64>,
        max: Option<f64>,
        line_width: f32,
        fill: bool,
    },
    Gauge {
        value: Binding,
        min: f64,
        max: f64,
        start_angle: f32,
        sweep_angle: f32,
        thickness: f32,
        needle_width: f32,
    },
    Conditional {
        condition: Condition,
        children: Vec<Widget>,
    },
}
