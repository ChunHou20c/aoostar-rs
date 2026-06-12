// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::binding::Binding;
use crate::error::DashboardError;
use crate::layout::LayoutTree;
use crate::renderer::Renderer;
use crate::style::StyleSheet;
use crate::widget::{
    Condition, ConditionComparison, FlexDirection, ProgressOrientation, Widget, WidgetKind,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Dashboard {
    source: PathBuf,
    options: DashboardOptions,
    stylesheet: StyleSheet,
    root: Widget,
}

impl Dashboard {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, DashboardError> {
        let requested_path = path.as_ref();
        let source = fs::canonicalize(requested_path).map_err(|source| DashboardError::Read {
            path: requested_path.to_path_buf(),
            source,
        })?;
        let contents = fs::read_to_string(&source).map_err(|error| DashboardError::Read {
            path: source.clone(),
            source: error,
        })?;
        let raw: RawDashboard =
            toml::from_str(&contents).map_err(|error| DashboardError::Parse {
                path: source.clone(),
                source: error,
            })?;

        raw.normalize(source)
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn options(&self) -> &DashboardOptions {
        &self.options
    }

    pub fn root(&self) -> &Widget {
        &self.root
    }

    pub fn stylesheet(&self) -> &StyleSheet {
        &self.stylesheet
    }

    pub fn compute_layout(&self) -> Result<LayoutTree, DashboardError> {
        Renderer::new(self)?.compute_layout(self)
    }

    pub fn asset_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![self.source.clone(), self.options.stylesheet.clone()];
        paths.extend(self.options.fonts.iter().cloned());
        collect_image_paths(&self.root, &mut paths);
        paths.sort();
        paths.dedup();
        paths
    }
}

fn collect_image_paths(widget: &Widget, paths: &mut Vec<PathBuf>) {
    if let WidgetKind::Image { source } = widget.kind() {
        paths.push(source.clone());
    }
    for child in widget.children() {
        collect_image_paths(child, paths);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DashboardOptions {
    width: u32,
    height: u32,
    stylesheet: PathBuf,
    background: Option<String>,
    fonts: Vec<PathBuf>,
}

impl DashboardOptions {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn stylesheet(&self) -> &Path {
        &self.stylesheet
    }

    pub fn background(&self) -> Option<&str> {
        self.background.as_deref()
    }

    pub fn fonts(&self) -> &[PathBuf] {
        &self.fonts
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDashboard {
    dashboard: RawDashboardOptions,
    #[serde(default)]
    components: HashMap<String, RawWidget>,
    root: RawWidget,
}

impl RawDashboard {
    fn normalize(self, source: PathBuf) -> Result<Dashboard, DashboardError> {
        let base_dir = source.parent().unwrap_or_else(|| Path::new("."));
        let options = self.dashboard.normalize(&source, base_dir)?;
        let stylesheet = StyleSheet::load(options.stylesheet())?;
        for (name, component) in &self.components {
            if !is_identifier(name) {
                return Err(DashboardError::validation(
                    &source,
                    format!("invalid component name {name:?}; use letters, digits, '_' or '-'"),
                ));
            }
            if component.contains_id() {
                return Err(DashboardError::validation(
                    &source,
                    format!(
                        "component {name:?} contains an id; reusable component templates cannot define ids"
                    ),
                ));
            }
        }
        let mut ids = HashSet::new();
        let root = self.root.normalize(
            &source,
            base_dir,
            "root".to_string(),
            &mut ids,
            &self.components,
            &mut Vec::new(),
        )?;

        Ok(Dashboard {
            source,
            options,
            stylesheet,
            root,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDashboardOptions {
    width: u32,
    height: u32,
    stylesheet: PathBuf,
    background: Option<String>,
    #[serde(default)]
    fonts: Vec<PathBuf>,
}

impl RawDashboardOptions {
    fn normalize(self, source: &Path, base_dir: &Path) -> Result<DashboardOptions, DashboardError> {
        if self.width == 0 || self.height == 0 {
            return Err(DashboardError::validation(
                source,
                "dashboard width and height must be greater than zero",
            ));
        }

        if let Some(background) = &self.background
            && !is_hex_color(background)
        {
            return Err(DashboardError::validation(
                source,
                format!("dashboard.background must be #RRGGBB or #RRGGBBAA, got {background:?}"),
            ));
        }

        let stylesheet =
            resolve_existing_file(source, base_dir, &self.stylesheet, "dashboard.stylesheet")?;
        let fonts = self
            .fonts
            .iter()
            .enumerate()
            .map(|(index, font)| {
                resolve_existing_file(source, base_dir, font, &format!("dashboard.fonts[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DashboardOptions {
            width: self.width,
            height: self.height,
            stylesheet,
            background: self.background,
            fonts,
        })
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum RawWidgetType {
    Row,
    Column,
    Stack,
    Text,
    Image,
    Spacer,
    Progress,
    CircularProgress,
    Graph,
    Gauge,
    Conditional,
    Component,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWidget {
    #[serde(rename = "type")]
    kind: RawWidgetType,
    id: Option<String>,
    #[serde(default, rename = "class")]
    classes: Vec<String>,
    #[serde(default)]
    children: Vec<RawWidget>,
    text: Option<String>,
    source: Option<PathBuf>,
    value: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    orientation: Option<RawProgressOrientation>,
    #[serde(rename = "start-angle")]
    start_angle: Option<f32>,
    #[serde(rename = "sweep-angle")]
    sweep_angle: Option<f32>,
    thickness: Option<f32>,
    #[serde(rename = "needle-width")]
    needle_width: Option<f32>,
    #[serde(rename = "line-width")]
    line_width: Option<f32>,
    fill: Option<bool>,
    equals: Option<String>,
    #[serde(rename = "not-equals")]
    not_equals: Option<String>,
    component: Option<String>,
}

impl RawWidget {
    fn contains_id(&self) -> bool {
        self.id.is_some() || self.children.iter().any(Self::contains_id)
    }

    fn expand_component(
        self,
        dashboard_path: &Path,
        base_dir: &Path,
        source_path: String,
        ids: &mut HashSet<String>,
        components: &HashMap<String, RawWidget>,
        component_stack: &mut Vec<String>,
    ) -> Result<Widget, DashboardError> {
        self.reject_children(dashboard_path, &source_path)?;
        self.reject_fields(
            dashboard_path,
            &source_path,
            [
                ("text", self.text.is_some()),
                ("source", self.source.is_some()),
                ("value", self.value.is_some()),
                ("min", self.min.is_some()),
                ("max", self.max.is_some()),
                ("orientation", self.orientation.is_some()),
                ("start-angle", self.start_angle.is_some()),
                ("sweep-angle", self.sweep_angle.is_some()),
                ("thickness", self.thickness.is_some()),
                ("needle-width", self.needle_width.is_some()),
                ("line-width", self.line_width.is_some()),
                ("fill", self.fill.is_some()),
                ("equals", self.equals.is_some()),
                ("not-equals", self.not_equals.is_some()),
            ],
        )?;
        let name = required_string(dashboard_path, &source_path, "component", self.component)?;
        if component_stack.contains(&name) {
            let mut cycle = component_stack.clone();
            cycle.push(name);
            return Err(widget_error(
                dashboard_path,
                &source_path,
                format!("component cycle detected: {}", cycle.join(" -> ")),
            ));
        }
        let mut template = components.get(&name).cloned().ok_or_else(|| {
            widget_error(
                dashboard_path,
                &source_path,
                format!("unknown component {name:?}"),
            )
        })?;
        template.id = self.id;
        template.classes.extend(self.classes);
        component_stack.push(name);
        let result = template.normalize(
            dashboard_path,
            base_dir,
            source_path,
            ids,
            components,
            component_stack,
        );
        component_stack.pop();
        result
    }

    fn normalize(
        self,
        dashboard_path: &Path,
        base_dir: &Path,
        source_path: String,
        ids: &mut HashSet<String>,
        components: &HashMap<String, RawWidget>,
        component_stack: &mut Vec<String>,
    ) -> Result<Widget, DashboardError> {
        if matches!(self.kind, RawWidgetType::Component) {
            return self.expand_component(
                dashboard_path,
                base_dir,
                source_path,
                ids,
                components,
                component_stack,
            );
        }
        self.validate_identity(dashboard_path, &source_path, ids)?;

        let kind = match self.kind {
            RawWidgetType::Row => {
                self.reject_content_fields(dashboard_path, &source_path)?;
                WidgetKind::Flex {
                    direction: FlexDirection::Row,
                    children: normalize_children(
                        self.children,
                        dashboard_path,
                        base_dir,
                        &source_path,
                        ids,
                        components,
                        component_stack,
                    )?,
                }
            }
            RawWidgetType::Column => {
                self.reject_content_fields(dashboard_path, &source_path)?;
                WidgetKind::Flex {
                    direction: FlexDirection::Column,
                    children: normalize_children(
                        self.children,
                        dashboard_path,
                        base_dir,
                        &source_path,
                        ids,
                        components,
                        component_stack,
                    )?,
                }
            }
            RawWidgetType::Stack => {
                self.reject_content_fields(dashboard_path, &source_path)?;
                WidgetKind::Stack {
                    children: normalize_children(
                        self.children,
                        dashboard_path,
                        base_dir,
                        &source_path,
                        ids,
                        components,
                        component_stack,
                    )?,
                }
            }
            RawWidgetType::Text => {
                self.reject_children(dashboard_path, &source_path)?;
                self.reject_fields(
                    dashboard_path,
                    &source_path,
                    [
                        ("source", self.source.is_some()),
                        ("value", self.value.is_some()),
                        ("min", self.min.is_some()),
                        ("max", self.max.is_some()),
                        ("orientation", self.orientation.is_some()),
                        ("start-angle", self.start_angle.is_some()),
                        ("sweep-angle", self.sweep_angle.is_some()),
                        ("thickness", self.thickness.is_some()),
                        ("needle-width", self.needle_width.is_some()),
                        ("line-width", self.line_width.is_some()),
                        ("fill", self.fill.is_some()),
                        ("equals", self.equals.is_some()),
                        ("not-equals", self.not_equals.is_some()),
                        ("component", self.component.is_some()),
                    ],
                )?;
                WidgetKind::Text {
                    text: parse_binding(
                        dashboard_path,
                        &source_path,
                        "text",
                        required_string(dashboard_path, &source_path, "text", self.text)?,
                    )?,
                }
            }
            RawWidgetType::Image => {
                self.reject_children(dashboard_path, &source_path)?;
                self.reject_fields(
                    dashboard_path,
                    &source_path,
                    [
                        ("text", self.text.is_some()),
                        ("value", self.value.is_some()),
                        ("min", self.min.is_some()),
                        ("max", self.max.is_some()),
                        ("orientation", self.orientation.is_some()),
                        ("start-angle", self.start_angle.is_some()),
                        ("sweep-angle", self.sweep_angle.is_some()),
                        ("thickness", self.thickness.is_some()),
                        ("needle-width", self.needle_width.is_some()),
                        ("line-width", self.line_width.is_some()),
                        ("fill", self.fill.is_some()),
                        ("equals", self.equals.is_some()),
                        ("not-equals", self.not_equals.is_some()),
                        ("component", self.component.is_some()),
                    ],
                )?;
                let configured_source = self.source.ok_or_else(|| {
                    widget_error(dashboard_path, &source_path, "image requires source")
                })?;
                WidgetKind::Image {
                    source: resolve_existing_file(
                        dashboard_path,
                        base_dir,
                        &configured_source,
                        &format!("{source_path}.source"),
                    )?,
                }
            }
            RawWidgetType::Spacer => {
                self.reject_children(dashboard_path, &source_path)?;
                self.reject_content_fields(dashboard_path, &source_path)?;
                WidgetKind::Spacer
            }
            RawWidgetType::Progress => {
                self.reject_children(dashboard_path, &source_path)?;
                self.reject_fields(
                    dashboard_path,
                    &source_path,
                    [
                        ("text", self.text.is_some()),
                        ("source", self.source.is_some()),
                        ("start-angle", self.start_angle.is_some()),
                        ("sweep-angle", self.sweep_angle.is_some()),
                        ("thickness", self.thickness.is_some()),
                        ("needle-width", self.needle_width.is_some()),
                        ("line-width", self.line_width.is_some()),
                        ("fill", self.fill.is_some()),
                        ("equals", self.equals.is_some()),
                        ("not-equals", self.not_equals.is_some()),
                        ("component", self.component.is_some()),
                    ],
                )?;
                let value = parse_binding(
                    dashboard_path,
                    &source_path,
                    "value",
                    required_string(dashboard_path, &source_path, "value", self.value)?,
                )?;
                let min = self.min.unwrap_or(0.0);
                let max = self.max.unwrap_or(100.0);
                if !min.is_finite() || !max.is_finite() || min >= max {
                    return Err(widget_error(
                        dashboard_path,
                        &source_path,
                        format!("progress range must be finite and min < max, got {min}..{max}"),
                    ));
                }
                WidgetKind::Progress {
                    value,
                    min,
                    max,
                    orientation: self.orientation.unwrap_or_default().into(),
                }
            }
            RawWidgetType::CircularProgress => {
                self.reject_children(dashboard_path, &source_path)?;
                self.reject_fields(
                    dashboard_path,
                    &source_path,
                    [
                        ("text", self.text.is_some()),
                        ("source", self.source.is_some()),
                        ("orientation", self.orientation.is_some()),
                        ("needle-width", self.needle_width.is_some()),
                        ("line-width", self.line_width.is_some()),
                        ("fill", self.fill.is_some()),
                        ("equals", self.equals.is_some()),
                        ("not-equals", self.not_equals.is_some()),
                        ("component", self.component.is_some()),
                    ],
                )?;
                let value = parse_binding(
                    dashboard_path,
                    &source_path,
                    "value",
                    required_string(dashboard_path, &source_path, "value", self.value)?,
                )?;
                let (min, max) = validate_range(
                    dashboard_path,
                    &source_path,
                    "circular-progress",
                    self.min.unwrap_or(0.0),
                    self.max.unwrap_or(100.0),
                )?;
                let (start_angle, sweep_angle, thickness) = validate_arc(
                    dashboard_path,
                    &source_path,
                    self.start_angle.unwrap_or(-90.0),
                    self.sweep_angle.unwrap_or(360.0),
                    self.thickness.unwrap_or(8.0),
                )?;
                WidgetKind::CircularProgress {
                    value,
                    min,
                    max,
                    start_angle,
                    sweep_angle,
                    thickness,
                }
            }
            RawWidgetType::Graph => {
                self.reject_children(dashboard_path, &source_path)?;
                self.reject_fields(
                    dashboard_path,
                    &source_path,
                    [
                        ("text", self.text.is_some()),
                        ("source", self.source.is_some()),
                        ("orientation", self.orientation.is_some()),
                        ("start-angle", self.start_angle.is_some()),
                        ("sweep-angle", self.sweep_angle.is_some()),
                        ("thickness", self.thickness.is_some()),
                        ("needle-width", self.needle_width.is_some()),
                        ("equals", self.equals.is_some()),
                        ("not-equals", self.not_equals.is_some()),
                        ("component", self.component.is_some()),
                    ],
                )?;
                if self.min.is_some_and(|value| !value.is_finite())
                    || self.max.is_some_and(|value| !value.is_finite())
                    || matches!((self.min, self.max), (Some(min), Some(max)) if min >= max)
                {
                    return Err(widget_error(
                        dashboard_path,
                        &source_path,
                        "graph min and max must be finite and min < max",
                    ));
                }
                let line_width = self.line_width.unwrap_or(2.0);
                validate_positive(dashboard_path, &source_path, "line-width", line_width)?;
                WidgetKind::Graph {
                    values: parse_binding(
                        dashboard_path,
                        &source_path,
                        "value",
                        required_string(dashboard_path, &source_path, "value", self.value)?,
                    )?,
                    min: self.min,
                    max: self.max,
                    line_width,
                    fill: self.fill.unwrap_or(false),
                }
            }
            RawWidgetType::Gauge => {
                self.reject_children(dashboard_path, &source_path)?;
                self.reject_fields(
                    dashboard_path,
                    &source_path,
                    [
                        ("text", self.text.is_some()),
                        ("source", self.source.is_some()),
                        ("orientation", self.orientation.is_some()),
                        ("line-width", self.line_width.is_some()),
                        ("fill", self.fill.is_some()),
                        ("equals", self.equals.is_some()),
                        ("not-equals", self.not_equals.is_some()),
                        ("component", self.component.is_some()),
                    ],
                )?;
                let value = parse_binding(
                    dashboard_path,
                    &source_path,
                    "value",
                    required_string(dashboard_path, &source_path, "value", self.value)?,
                )?;
                let (min, max) = validate_range(
                    dashboard_path,
                    &source_path,
                    "gauge",
                    self.min.unwrap_or(0.0),
                    self.max.unwrap_or(100.0),
                )?;
                let (start_angle, sweep_angle, thickness) = validate_arc(
                    dashboard_path,
                    &source_path,
                    self.start_angle.unwrap_or(-135.0),
                    self.sweep_angle.unwrap_or(270.0),
                    self.thickness.unwrap_or(8.0),
                )?;
                let needle_width = self.needle_width.unwrap_or(3.0);
                validate_positive(dashboard_path, &source_path, "needle-width", needle_width)?;
                WidgetKind::Gauge {
                    value,
                    min,
                    max,
                    start_angle,
                    sweep_angle,
                    thickness,
                    needle_width,
                }
            }
            RawWidgetType::Conditional => {
                self.reject_fields(
                    dashboard_path,
                    &source_path,
                    [
                        ("text", self.text.is_some()),
                        ("source", self.source.is_some()),
                        ("min", self.min.is_some()),
                        ("max", self.max.is_some()),
                        ("orientation", self.orientation.is_some()),
                        ("start-angle", self.start_angle.is_some()),
                        ("sweep-angle", self.sweep_angle.is_some()),
                        ("thickness", self.thickness.is_some()),
                        ("needle-width", self.needle_width.is_some()),
                        ("line-width", self.line_width.is_some()),
                        ("fill", self.fill.is_some()),
                        ("component", self.component.is_some()),
                    ],
                )?;
                if self.equals.is_some() && self.not_equals.is_some() {
                    return Err(widget_error(
                        dashboard_path,
                        &source_path,
                        "conditional accepts either equals or not-equals, not both",
                    ));
                }
                let condition = Condition::new(
                    parse_binding(
                        dashboard_path,
                        &source_path,
                        "value",
                        required_string(dashboard_path, &source_path, "value", self.value)?,
                    )?,
                    match (self.equals, self.not_equals) {
                        (Some(value), None) => ConditionComparison::Equals(value),
                        (None, Some(value)) => ConditionComparison::NotEquals(value),
                        (None, None) => ConditionComparison::Truthy,
                        (Some(_), Some(_)) => unreachable!("validated above"),
                    },
                );
                WidgetKind::Conditional {
                    condition,
                    children: normalize_children(
                        self.children,
                        dashboard_path,
                        base_dir,
                        &source_path,
                        ids,
                        components,
                        component_stack,
                    )?,
                }
            }
            RawWidgetType::Component => {
                unreachable!("components are expanded before normalization")
            }
        };

        Ok(Widget::new(source_path, self.id, self.classes, kind))
    }

    fn validate_identity(
        &self,
        dashboard_path: &Path,
        source_path: &str,
        ids: &mut HashSet<String>,
    ) -> Result<(), DashboardError> {
        if let Some(id) = &self.id {
            if !is_identifier(id) {
                return Err(widget_error(
                    dashboard_path,
                    source_path,
                    format!("invalid id {id:?}; use letters, digits, '_' or '-'"),
                ));
            }
            if !ids.insert(id.clone()) {
                return Err(widget_error(
                    dashboard_path,
                    source_path,
                    format!("duplicate widget id {id:?}"),
                ));
            }
        }

        for class in &self.classes {
            if !is_identifier(class) {
                return Err(widget_error(
                    dashboard_path,
                    source_path,
                    format!("invalid class {class:?}; use letters, digits, '_' or '-'"),
                ));
            }
        }

        Ok(())
    }

    fn reject_children(
        &self,
        dashboard_path: &Path,
        source_path: &str,
    ) -> Result<(), DashboardError> {
        if self.children.is_empty() {
            Ok(())
        } else {
            Err(widget_error(
                dashboard_path,
                source_path,
                "this widget cannot contain children",
            ))
        }
    }

    fn reject_content_fields(
        &self,
        dashboard_path: &Path,
        source_path: &str,
    ) -> Result<(), DashboardError> {
        self.reject_fields(
            dashboard_path,
            source_path,
            [
                ("text", self.text.is_some()),
                ("source", self.source.is_some()),
                ("value", self.value.is_some()),
                ("min", self.min.is_some()),
                ("max", self.max.is_some()),
                ("orientation", self.orientation.is_some()),
                ("start-angle", self.start_angle.is_some()),
                ("sweep-angle", self.sweep_angle.is_some()),
                ("thickness", self.thickness.is_some()),
                ("needle-width", self.needle_width.is_some()),
                ("line-width", self.line_width.is_some()),
                ("fill", self.fill.is_some()),
                ("equals", self.equals.is_some()),
                ("not-equals", self.not_equals.is_some()),
                ("component", self.component.is_some()),
            ],
        )
    }

    fn reject_fields<const N: usize>(
        &self,
        dashboard_path: &Path,
        source_path: &str,
        fields: [(&str, bool); N],
    ) -> Result<(), DashboardError> {
        if let Some((field, _)) = fields.into_iter().find(|(_, present)| *present) {
            Err(widget_error(
                dashboard_path,
                source_path,
                format!("field {field:?} is not valid for this widget type"),
            ))
        } else {
            Ok(())
        }
    }
}

fn normalize_children(
    children: Vec<RawWidget>,
    dashboard_path: &Path,
    base_dir: &Path,
    parent_path: &str,
    ids: &mut HashSet<String>,
    components: &HashMap<String, RawWidget>,
    component_stack: &mut Vec<String>,
) -> Result<Vec<Widget>, DashboardError> {
    children
        .into_iter()
        .enumerate()
        .map(|(index, child)| {
            child.normalize(
                dashboard_path,
                base_dir,
                format!("{parent_path}.children[{index}]"),
                ids,
                components,
                component_stack,
            )
        })
        .collect()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum RawProgressOrientation {
    #[default]
    Horizontal,
    Vertical,
}

fn validate_range(
    dashboard_path: &Path,
    widget_path: &str,
    widget_type: &str,
    min: f64,
    max: f64,
) -> Result<(f64, f64), DashboardError> {
    if !min.is_finite() || !max.is_finite() || min >= max {
        Err(widget_error(
            dashboard_path,
            widget_path,
            format!("{widget_type} range must be finite and min < max, got {min}..{max}"),
        ))
    } else {
        Ok((min, max))
    }
}

fn validate_arc(
    dashboard_path: &Path,
    widget_path: &str,
    start_angle: f32,
    sweep_angle: f32,
    thickness: f32,
) -> Result<(f32, f32, f32), DashboardError> {
    if !start_angle.is_finite()
        || !sweep_angle.is_finite()
        || sweep_angle == 0.0
        || sweep_angle.abs() > 360.0
    {
        return Err(widget_error(
            dashboard_path,
            widget_path,
            "angles must be finite and sweep-angle must be non-zero and at most 360 degrees",
        ));
    }
    validate_positive(dashboard_path, widget_path, "thickness", thickness)?;
    Ok((start_angle, sweep_angle, thickness))
}

fn validate_positive(
    dashboard_path: &Path,
    widget_path: &str,
    field: &str,
    value: f32,
) -> Result<(), DashboardError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(widget_error(
            dashboard_path,
            widget_path,
            format!("{field} must be finite and greater than zero"),
        ))
    }
}

impl From<RawProgressOrientation> for ProgressOrientation {
    fn from(value: RawProgressOrientation) -> Self {
        match value {
            RawProgressOrientation::Horizontal => Self::Horizontal,
            RawProgressOrientation::Vertical => Self::Vertical,
        }
    }
}

fn required_string(
    dashboard_path: &Path,
    widget_path: &str,
    field: &str,
    value: Option<String>,
) -> Result<String, DashboardError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(widget_error(
            dashboard_path,
            widget_path,
            format!("widget requires non-empty {field}"),
        )),
    }
}

fn parse_binding(
    dashboard_path: &Path,
    widget_path: &str,
    field: &str,
    value: String,
) -> Result<Binding, DashboardError> {
    Binding::parse(&value).map_err(|error| {
        widget_error(
            dashboard_path,
            widget_path,
            format!("invalid {field} binding: {error}"),
        )
    })
}

fn resolve_existing_file(
    dashboard_path: &Path,
    base_dir: &Path,
    configured_path: &Path,
    field: &str,
) -> Result<PathBuf, DashboardError> {
    let path = if configured_path.is_absolute() {
        configured_path.to_path_buf()
    } else {
        base_dir.join(configured_path)
    };

    let canonical_path = fs::canonicalize(&path).map_err(|error| {
        DashboardError::validation(
            dashboard_path,
            format!("{field} references unreadable file {path:?}: {error}"),
        )
    })?;

    if !canonical_path.is_file() {
        return Err(DashboardError::validation(
            dashboard_path,
            format!("{field} must reference a file, got {canonical_path:?}"),
        ));
    }

    Ok(canonical_path)
}

fn widget_error(
    dashboard_path: &Path,
    widget_path: &str,
    message: impl Into<String>,
) -> DashboardError {
    DashboardError::validation(dashboard_path, format!("{widget_path}: {}", message.into()))
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn is_hex_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn dashboard_fixture(contents: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("dashboard.css"), ".dashboard {}\n").unwrap();
        let dashboard_path = dir.path().join("dashboard.toml");
        let mut dashboard = fs::File::create(&dashboard_path).unwrap();
        dashboard.write_all(contents.as_bytes()).unwrap();
        (dir, dashboard_path)
    }

    #[test]
    fn loads_and_normalizes_nested_widgets() {
        let (_dir, path) = dashboard_fixture(
            r##"
[dashboard]
width = 960
height = 376
stylesheet = "dashboard.css"
background = "#101318"

[root]
type = "row"
id = "dashboard"
class = ["dashboard"]

[[root.children]]
type = "text"
text = "CPU"

[[root.children]]
type = "progress"
value = "{{ cpu_percent }}"
"##,
        );

        let dashboard = Dashboard::load(path).unwrap();
        assert_eq!(dashboard.options().width(), 960);
        assert_eq!(dashboard.options().height(), 376);
        assert!(dashboard.options().stylesheet().is_absolute());
        assert_eq!(dashboard.root().source_path(), "root");

        let WidgetKind::Flex {
            direction,
            children,
        } = dashboard.root().kind()
        else {
            panic!("root should normalize to a flex widget");
        };
        assert_eq!(*direction, FlexDirection::Row);
        assert_eq!(children.len(), 2);
        assert_eq!(children[1].source_path(), "root.children[1]");
    }

    #[test]
    fn rejects_unknown_configuration_fields() {
        let (_dir, path) = dashboard_fixture(
            r#"
[dashboard]
width = 960
height = 376
stylesheet = "dashboard.css"
unknown = true

[root]
type = "spacer"
"#,
        );

        let error = Dashboard::load(path).unwrap_err().to_string();
        assert!(error.contains("unknown field `unknown`"), "{error}");
    }

    #[test]
    fn rejects_fields_for_the_wrong_widget_type() {
        let (_dir, path) = dashboard_fixture(
            r#"
[dashboard]
width = 960
height = 376
stylesheet = "dashboard.css"

[root]
type = "row"
text = "not valid"
"#,
        );

        let error = Dashboard::load(path).unwrap_err().to_string();
        assert!(error.contains("root"), "{error}");
        assert!(error.contains("field \"text\" is not valid"), "{error}");
    }

    #[test]
    fn rejects_duplicate_widget_ids() {
        let (_dir, path) = dashboard_fixture(
            r#"
[dashboard]
width = 960
height = 376
stylesheet = "dashboard.css"

[root]
type = "row"

[[root.children]]
type = "spacer"
id = "duplicate"

[[root.children]]
type = "spacer"
id = "duplicate"
"#,
        );

        let error = Dashboard::load(path).unwrap_err().to_string();
        assert!(error.contains("root.children[1]"), "{error}");
        assert!(error.contains("duplicate widget id"), "{error}");
    }

    #[test]
    fn rejects_invalid_progress_range() {
        let (_dir, path) = dashboard_fixture(
            r#"
[dashboard]
width = 960
height = 376
stylesheet = "dashboard.css"

[root]
type = "progress"
value = "50"
min = 100
max = 0
"#,
        );

        let error = Dashboard::load(path).unwrap_err().to_string();
        assert!(error.contains("progress range"), "{error}");
    }

    #[test]
    fn rejects_invalid_stylesheet() {
        let (dir, path) = dashboard_fixture(
            r#"
[dashboard]
width = 960
height = 376
stylesheet = "dashboard.css"

[root]
type = "spacer"
"#,
        );
        fs::write(
            dir.path().join("dashboard.css"),
            "spacer { unsupported: true; }",
        )
        .unwrap();

        let error = Dashboard::load(path).unwrap_err().to_string();
        assert!(error.contains("failed to parse stylesheet"), "{error}");
        assert!(error.contains("unsupported property"), "{error}");
    }
}
