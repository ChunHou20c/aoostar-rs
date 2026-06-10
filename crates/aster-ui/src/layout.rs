// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::ValueMap;
use crate::error::DashboardError;
use crate::renderer::AssetCache;
use crate::style::{
    Align, ComputedStyle, Display, Edges, FlexDirectionStyle, JustifyContent, Length, Overflow,
    StyleSheet,
};
use crate::widget::{Widget, WidgetKind};
use taffy::Point;
use taffy::prelude::{
    AlignItems, AvailableSpace, Dimension, Display as TaffyDisplay,
    FlexDirection as TaffyFlexDirection, JustifyContent as TaffyJustifyContent, LengthPercentage,
    LengthPercentageAuto, NodeId, Position, Rect, Size, Style, TaffyTree,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutTree {
    root: LayoutNode,
}

impl LayoutTree {
    #[cfg(test)]
    pub(crate) fn compute(
        root: &Widget,
        stylesheet: &StyleSheet,
        width: u32,
        height: u32,
    ) -> Result<Self, DashboardError> {
        let mut taffy = TaffyTree::<()>::new();
        let built = build_node(
            &mut taffy, root, stylesheet, None, None, true, width, height,
        )?;
        taffy
            .compute_layout(
                built.node,
                Size {
                    width: AvailableSpace::Definite(width as f32),
                    height: AvailableSpace::Definite(height as f32),
                },
            )
            .map_err(|error| DashboardError::layout(error.to_string()))?;

        Ok(Self {
            root: collect_layout(&taffy, built, 0.0, 0.0)?,
        })
    }

    pub(crate) fn compute_with_assets(
        dashboard_source: &std::path::Path,
        root: &Widget,
        stylesheet: &StyleSheet,
        width: u32,
        height: u32,
        assets: &mut AssetCache,
        values: &ValueMap,
    ) -> Result<Self, DashboardError> {
        let mut taffy = TaffyTree::<MeasureContext>::new();
        let built = build_measured_node(
            &mut taffy,
            dashboard_source,
            root,
            stylesheet,
            None,
            None,
            true,
            width,
            height,
            values,
        )?;
        let mut measure_error = None;
        taffy
            .compute_layout_with_measure(
                built.node,
                Size {
                    width: AvailableSpace::Definite(width as f32),
                    height: AvailableSpace::Definite(height as f32),
                },
                |known, available, _, context, _| {
                    let result = match context {
                        Some(MeasureContext::Text { text, style }) => {
                            assets.measure_text(text, style, known, available)
                        }
                        Some(MeasureContext::Image { source }) => {
                            assets.measure_image(source, known)
                        }
                        None => Ok(Size::ZERO),
                    };
                    match result {
                        Ok(size) => size,
                        Err(error) => {
                            measure_error = Some(error);
                            Size::ZERO
                        }
                    }
                },
            )
            .map_err(|error| DashboardError::layout(error.to_string()))?;
        if let Some(error) = measure_error {
            return Err(error);
        }

        Ok(Self {
            root: collect_measured_layout(&taffy, built, 0.0, 0.0)?,
        })
    }

    pub fn root(&self) -> &LayoutNode {
        &self.root
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    source_path: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    content_x: f32,
    content_y: f32,
    content_width: f32,
    content_height: f32,
    style: ComputedStyle,
    children: Vec<LayoutNode>,
}

impl LayoutNode {
    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn content_x(&self) -> f32 {
        self.content_x
    }

    pub fn content_y(&self) -> f32 {
        self.content_y
    }

    pub fn content_width(&self) -> f32 {
        self.content_width
    }

    pub fn content_height(&self) -> f32 {
        self.content_height
    }

    pub fn style(&self) -> &ComputedStyle {
        &self.style
    }

    pub fn children(&self) -> &[LayoutNode] {
        &self.children
    }

    pub fn find(&self, source_path: &str) -> Option<&Self> {
        if self.source_path == source_path {
            return Some(self);
        }
        self.children
            .iter()
            .find_map(|child| child.find(source_path))
    }
}

struct BuiltNode {
    node: NodeId,
    source_path: String,
    children: Vec<BuiltNode>,
    style: ComputedStyle,
}

#[derive(Debug)]
enum MeasureContext {
    Text { text: String, style: ComputedStyle },
    Image { source: std::path::PathBuf },
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
fn build_node(
    taffy: &mut TaffyTree<()>,
    widget: &Widget,
    stylesheet: &StyleSheet,
    parent_style: Option<&ComputedStyle>,
    stack_inset: Option<Edges>,
    is_root: bool,
    display_width: u32,
    display_height: u32,
) -> Result<BuiltNode, DashboardError> {
    let computed = stylesheet.compute(widget, parent_style);
    let is_stack = computed.display == Display::Stack;
    let children = widget
        .children()
        .iter()
        .map(|child| {
            build_node(
                taffy,
                child,
                stylesheet,
                Some(&computed),
                is_stack.then_some(computed.padding),
                false,
                display_width,
                display_height,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let child_ids: Vec<_> = children.iter().map(|child| child.node).collect();
    let style = to_taffy_style(
        &computed,
        stack_inset,
        is_root,
        display_width,
        display_height,
    );
    let node = if child_ids.is_empty() {
        taffy.new_leaf(style)
    } else {
        taffy.new_with_children(style, &child_ids)
    }
    .map_err(|error| DashboardError::layout(error.to_string()))?;

    Ok(BuiltNode {
        node,
        source_path: widget.source_path().to_string(),
        children,
        style: computed,
    })
}

#[cfg(test)]
fn collect_layout(
    taffy: &TaffyTree<()>,
    built: BuiltNode,
    parent_x: f32,
    parent_y: f32,
) -> Result<LayoutNode, DashboardError> {
    let layout = taffy
        .layout(built.node)
        .map_err(|error| DashboardError::layout(error.to_string()))?;
    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    let children = built
        .children
        .into_iter()
        .map(|child| collect_layout(taffy, child, x, y))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LayoutNode {
        source_path: built.source_path,
        x,
        y,
        width: layout.size.width,
        height: layout.size.height,
        content_x: x + layout.border.left + layout.padding.left,
        content_y: y + layout.border.top + layout.padding.top,
        content_width: (layout.size.width
            - layout.border.left
            - layout.border.right
            - layout.padding.left
            - layout.padding.right)
            .max(0.0),
        content_height: (layout.size.height
            - layout.border.top
            - layout.border.bottom
            - layout.padding.top
            - layout.padding.bottom)
            .max(0.0),
        style: built.style,
        children,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_measured_node(
    taffy: &mut TaffyTree<MeasureContext>,
    dashboard_source: &std::path::Path,
    widget: &Widget,
    stylesheet: &StyleSheet,
    parent_style: Option<&ComputedStyle>,
    stack_inset: Option<Edges>,
    is_root: bool,
    display_width: u32,
    display_height: u32,
    values: &ValueMap,
) -> Result<BuiltNode, DashboardError> {
    let computed = stylesheet.compute(widget, parent_style);
    let is_stack = computed.display == Display::Stack;
    let children = widget
        .children()
        .iter()
        .map(|child| {
            build_measured_node(
                taffy,
                dashboard_source,
                child,
                stylesheet,
                Some(&computed),
                is_stack.then_some(computed.padding),
                false,
                display_width,
                display_height,
                values,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let child_ids: Vec<_> = children.iter().map(|child| child.node).collect();
    let style = to_taffy_style(
        &computed,
        stack_inset,
        is_root,
        display_width,
        display_height,
    );
    let context = match widget.kind() {
        WidgetKind::Text { text } => Some(MeasureContext::Text {
            text: text.resolve(values).map_err(|error| {
                DashboardError::binding(dashboard_source, widget.source_path(), error.to_string())
            })?,
            style: computed.clone(),
        }),
        WidgetKind::Image { source } => Some(MeasureContext::Image {
            source: source.clone(),
        }),
        _ => None,
    };
    let node = if child_ids.is_empty() {
        if let Some(context) = context {
            taffy.new_leaf_with_context(style, context)
        } else {
            taffy.new_leaf(style)
        }
    } else {
        taffy.new_with_children(style, &child_ids)
    }
    .map_err(|error| DashboardError::layout(error.to_string()))?;

    Ok(BuiltNode {
        node,
        source_path: widget.source_path().to_string(),
        children,
        style: computed,
    })
}

fn collect_measured_layout(
    taffy: &TaffyTree<MeasureContext>,
    built: BuiltNode,
    parent_x: f32,
    parent_y: f32,
) -> Result<LayoutNode, DashboardError> {
    let layout = taffy
        .layout(built.node)
        .map_err(|error| DashboardError::layout(error.to_string()))?;
    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    let children = built
        .children
        .into_iter()
        .map(|child| collect_measured_layout(taffy, child, x, y))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LayoutNode {
        source_path: built.source_path,
        x,
        y,
        width: layout.size.width,
        height: layout.size.height,
        content_x: x + layout.border.left + layout.padding.left,
        content_y: y + layout.border.top + layout.padding.top,
        content_width: (layout.size.width
            - layout.border.left
            - layout.border.right
            - layout.padding.left
            - layout.padding.right)
            .max(0.0),
        content_height: (layout.size.height
            - layout.border.top
            - layout.border.bottom
            - layout.padding.top
            - layout.padding.bottom)
            .max(0.0),
        style: built.style,
        children,
    })
}

fn to_taffy_style(
    style: &ComputedStyle,
    stack_inset: Option<Edges>,
    is_root: bool,
    display_width: u32,
    display_height: u32,
) -> Style {
    let mut width = dimension(style.width);
    let mut height = dimension(style.height);
    if is_root {
        width = Dimension::length(display_width as f32);
        height = Dimension::length(display_height as f32);
    }

    Style {
        display: match style.display {
            Display::Flex | Display::Stack => TaffyDisplay::Flex,
            Display::None => TaffyDisplay::None,
        },
        position: if stack_inset.is_some() {
            Position::Absolute
        } else {
            Position::Relative
        },
        inset: if let Some(inset) = stack_inset {
            Rect {
                left: length_percentage_auto(inset.left),
                right: length_percentage_auto(inset.right),
                top: length_percentage_auto(inset.top),
                bottom: length_percentage_auto(inset.bottom),
            }
        } else {
            Rect::auto()
        },
        size: Size { width, height },
        min_size: Size {
            width: dimension(style.min_width),
            height: dimension(style.min_height),
        },
        max_size: Size {
            width: dimension(style.max_width),
            height: dimension(style.max_height),
        },
        margin: edge_auto(style.margin),
        padding: edge_length(style.padding),
        border: Rect::length(style.border_width),
        align_items: Some(align(style.align_items)),
        align_self: style.align_self.map(align),
        justify_content: Some(justify(style.justify_content)),
        gap: Size {
            width: length_percentage(style.gap),
            height: length_percentage(style.gap),
        },
        flex_direction: match style.flex_direction {
            FlexDirectionStyle::Row => TaffyFlexDirection::Row,
            FlexDirectionStyle::Column => TaffyFlexDirection::Column,
        },
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        overflow: Point {
            x: overflow(style.overflow),
            y: overflow(style.overflow),
        },
        ..Default::default()
    }
}

fn dimension(value: Length) -> Dimension {
    match value {
        Length::Auto => Dimension::auto(),
        Length::Px(value) => Dimension::length(value),
        Length::Percent(value) => Dimension::percent(value),
    }
}

fn length_percentage(value: Length) -> LengthPercentage {
    match value {
        Length::Px(value) => LengthPercentage::length(value),
        Length::Percent(value) => LengthPercentage::percent(value),
        Length::Auto => LengthPercentage::length(0.0),
    }
}

fn length_percentage_auto(value: Length) -> LengthPercentageAuto {
    match value {
        Length::Auto => LengthPercentageAuto::auto(),
        Length::Px(value) => LengthPercentageAuto::length(value),
        Length::Percent(value) => LengthPercentageAuto::percent(value),
    }
}

fn edge_auto(edges: Edges) -> Rect<LengthPercentageAuto> {
    Rect {
        left: length_percentage_auto(edges.left),
        right: length_percentage_auto(edges.right),
        top: length_percentage_auto(edges.top),
        bottom: length_percentage_auto(edges.bottom),
    }
}

fn edge_length(edges: Edges) -> Rect<LengthPercentage> {
    Rect {
        left: length_percentage(edges.left),
        right: length_percentage(edges.right),
        top: length_percentage(edges.top),
        bottom: length_percentage(edges.bottom),
    }
}

fn align(value: Align) -> AlignItems {
    match value {
        Align::Start => AlignItems::Start,
        Align::End => AlignItems::End,
        Align::Center => AlignItems::Center,
        Align::Stretch => AlignItems::Stretch,
    }
}

fn justify(value: JustifyContent) -> TaffyJustifyContent {
    match value {
        JustifyContent::Start => TaffyJustifyContent::Start,
        JustifyContent::End => TaffyJustifyContent::End,
        JustifyContent::Center => TaffyJustifyContent::Center,
        JustifyContent::SpaceBetween => TaffyJustifyContent::SpaceBetween,
        JustifyContent::SpaceAround => TaffyJustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly => TaffyJustifyContent::SpaceEvenly,
    }
}

fn overflow(value: Overflow) -> taffy::Overflow {
    match value {
        Overflow::Visible => taffy::Overflow::Visible,
        Overflow::Hidden => taffy::Overflow::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::StyleSheet;
    use crate::widget::{FlexDirection, WidgetKind};

    fn widget(path: &str, kind: WidgetKind, id: Option<&str>, classes: &[&str]) -> Widget {
        Widget::new(
            path.to_string(),
            id.map(str::to_string),
            classes.iter().map(|class| class.to_string()).collect(),
            kind,
        )
    }

    #[test]
    fn computes_flex_sizes_padding_and_gap() {
        let children = vec![
            widget("root.children[0]", WidgetKind::Spacer, None, &["fixed"]),
            widget("root.children[1]", WidgetKind::Spacer, None, &["grow"]),
        ];
        let root = widget(
            "root",
            WidgetKind::Flex {
                direction: FlexDirection::Row,
                children,
            },
            None,
            &["root"],
        );
        let stylesheet = StyleSheet::parse(
            "test.css",
            r#"
.root { padding: 10px; gap: 5px; }
.fixed { width: 100px; }
.grow { flex-grow: 1; }
"#,
        )
        .unwrap();

        let layout = LayoutTree::compute(&root, &stylesheet, 300, 100).unwrap();
        let fixed = layout.root().find("root.children[0]").unwrap();
        let grow = layout.root().find("root.children[1]").unwrap();

        assert_eq!((fixed.x(), fixed.y()), (10.0, 10.0));
        assert_eq!((fixed.width(), fixed.height()), (100.0, 80.0));
        assert_eq!((grow.x(), grow.y()), (115.0, 10.0));
        assert_eq!((grow.width(), grow.height()), (175.0, 80.0));
    }

    #[test]
    fn stack_children_share_the_same_bounds() {
        let children = vec![
            widget("root.children[0]", WidgetKind::Spacer, None, &[]),
            widget("root.children[1]", WidgetKind::Spacer, None, &[]),
        ];
        let root = widget("root", WidgetKind::Stack { children }, None, &["root"]);
        let stylesheet = StyleSheet::parse("test.css", ".root { padding: 8px; }").unwrap();

        let layout = LayoutTree::compute(&root, &stylesheet, 200, 80).unwrap();
        let first = layout.root().find("root.children[0]").unwrap();
        let second = layout.root().find("root.children[1]").unwrap();

        assert_eq!(
            (first.x(), first.y(), first.width(), first.height()),
            (8.0, 8.0, 184.0, 64.0)
        );
        assert_eq!(
            (second.x(), second.y(), second.width(), second.height()),
            (8.0, 8.0, 184.0, 64.0)
        );
    }

    #[test]
    fn display_none_collapses_a_node() {
        let child = widget("root.children[0]", WidgetKind::Spacer, None, &["hidden"]);
        let root = widget(
            "root",
            WidgetKind::Flex {
                direction: FlexDirection::Row,
                children: vec![child],
            },
            None,
            &[],
        );
        let stylesheet = StyleSheet::parse("test.css", ".hidden { display: none; }").unwrap();

        let layout = LayoutTree::compute(&root, &stylesheet, 100, 50).unwrap();
        let hidden = layout.root().find("root.children[0]").unwrap();
        assert_eq!((hidden.width(), hidden.height()), (0.0, 0.0));
    }
}
