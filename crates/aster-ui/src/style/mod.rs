// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

mod parser;

use crate::error::DashboardError;
use crate::widget::{FlexDirection, Widget, WidgetKind};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    };

    pub const WHITE: Self = Self {
        red: 255,
        green: 255,
        blue: 255,
        alpha: 255,
    };
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum Display {
    #[default]
    Flex,
    Stack,
    None,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum FlexDirectionStyle {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum Align {
    Start,
    End,
    Center,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum TextAlign {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum TextOverflow {
    #[default]
    Clip,
    Ellipsis,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum WhiteSpace {
    #[default]
    Normal,
    NoWrap,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum ObjectFit {
    Fill,
    #[default]
    Contain,
    Cover,
    None,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum ObjectPosition {
    Start,
    #[default]
    Center,
    End,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Edges {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl Edges {
    fn all(value: Length) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub display: Display,
    pub flex_direction: FlexDirectionStyle,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
    pub gap: Length,
    pub margin: Edges,
    pub padding: Edges,
    pub align_items: Align,
    pub align_self: Option<Align>,
    pub justify_content: JustifyContent,
    pub color: Color,
    pub background_color: Color,
    pub opacity: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub border_radius: f32,
    pub overflow: Overflow,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub line_height: f32,
    pub text_align: TextAlign,
    pub text_overflow: TextOverflow,
    pub white_space: WhiteSpace,
    pub object_fit: ObjectFit,
    pub object_position: ObjectPosition,
}

impl ComputedStyle {
    fn initial(widget: &Widget, parent: Option<&Self>) -> Self {
        let (display, flex_direction) = match widget.kind() {
            WidgetKind::Flex {
                direction: FlexDirection::Row,
                ..
            } => (Display::Flex, FlexDirectionStyle::Row),
            WidgetKind::Flex {
                direction: FlexDirection::Column,
                ..
            } => (Display::Flex, FlexDirectionStyle::Column),
            WidgetKind::Stack { .. } => (Display::Stack, FlexDirectionStyle::Row),
            _ => (Display::Flex, FlexDirectionStyle::Row),
        };

        Self {
            display,
            flex_direction,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            min_height: Length::Auto,
            max_width: Length::Auto,
            max_height: Length::Auto,
            gap: Length::Px(0.0),
            margin: Edges::all(Length::Px(0.0)),
            padding: Edges::all(Length::Px(0.0)),
            align_items: Align::Stretch,
            align_self: None,
            justify_content: JustifyContent::Start,
            color: parent.map_or(Color::WHITE, |style| style.color),
            background_color: Color::TRANSPARENT,
            opacity: 1.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            border_radius: 0.0,
            overflow: Overflow::Visible,
            font_family: parent
                .map(|style| style.font_family.clone())
                .unwrap_or_else(|| "DejaVu Sans".to_string()),
            font_size: parent.map_or(16.0, |style| style.font_size),
            font_weight: parent.map_or(400, |style| style.font_weight),
            line_height: parent.map_or(1.2, |style| style.line_height),
            text_align: parent.map_or(TextAlign::Start, |style| style.text_align),
            text_overflow: TextOverflow::Clip,
            white_space: WhiteSpace::Normal,
            object_fit: ObjectFit::Contain,
            object_position: ObjectPosition::Center,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StyledNode {
    pub source_path: String,
    pub style: ComputedStyle,
    pub children: Vec<StyledNode>,
}

#[derive(Debug, Clone)]
pub struct StyleSheet {
    source: PathBuf,
    rules: Vec<Rule>,
}

impl StyleSheet {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, DashboardError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|source| DashboardError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(path, &contents)
    }

    pub fn parse(path: impl AsRef<Path>, contents: &str) -> Result<Self, DashboardError> {
        let source = path.as_ref().to_path_buf();
        let rules = parser::parse_stylesheet(&source, contents)?;
        Ok(Self { source, rules })
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn compute(&self, widget: &Widget, parent: Option<&ComputedStyle>) -> ComputedStyle {
        let mut computed = ComputedStyle::initial(widget, parent);
        let mut matching_rules: Vec<_> = self
            .rules
            .iter()
            .filter(|rule| rule.selector.matches(widget))
            .collect();
        matching_rules.sort_by_key(|rule| (rule.selector.specificity(), rule.order));

        for rule in matching_rules {
            rule.declarations.apply(&mut computed);
        }

        computed
    }

    pub fn compute_tree(&self, root: &Widget) -> StyledNode {
        self.compute_node(root, None)
    }

    fn compute_node(&self, widget: &Widget, parent: Option<&ComputedStyle>) -> StyledNode {
        let style = self.compute(widget, parent);
        let children = widget
            .children()
            .iter()
            .map(|child| self.compute_node(child, Some(&style)))
            .collect();

        StyledNode {
            source_path: widget.source_path().to_string(),
            style,
            children,
        }
    }
}

#[derive(Debug, Clone)]
struct Rule {
    selector: Selector,
    declarations: Declarations,
    order: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Selector {
    Type(String),
    Class(String),
    TypeClass { widget_type: String, class: String },
    Id(String),
}

impl Selector {
    fn matches(&self, widget: &Widget) -> bool {
        match self {
            Self::Type(widget_type) => widget.type_name() == widget_type,
            Self::Class(class) => widget.classes().iter().any(|candidate| candidate == class),
            Self::TypeClass { widget_type, class } => {
                widget.type_name() == widget_type
                    && widget.classes().iter().any(|candidate| candidate == class)
            }
            Self::Id(id) => widget.id() == Some(id.as_str()),
        }
    }

    fn specificity(&self) -> u8 {
        match self {
            Self::Type(_) => 1,
            Self::Class(_) => 2,
            Self::TypeClass { .. } => 3,
            Self::Id(_) => 4,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Declarations {
    display: Option<Display>,
    flex_direction: Option<FlexDirectionStyle>,
    flex_grow: Option<f32>,
    flex_shrink: Option<f32>,
    width: Option<Length>,
    height: Option<Length>,
    min_width: Option<Length>,
    min_height: Option<Length>,
    max_width: Option<Length>,
    max_height: Option<Length>,
    gap: Option<Length>,
    margin: Option<Edges>,
    padding: Option<Edges>,
    align_items: Option<Align>,
    align_self: Option<Align>,
    justify_content: Option<JustifyContent>,
    color: Option<Color>,
    background_color: Option<Color>,
    opacity: Option<f32>,
    border_width: Option<f32>,
    border_color: Option<Color>,
    border_radius: Option<f32>,
    overflow: Option<Overflow>,
    font_family: Option<String>,
    font_size: Option<f32>,
    font_weight: Option<u16>,
    line_height: Option<f32>,
    text_align: Option<TextAlign>,
    text_overflow: Option<TextOverflow>,
    white_space: Option<WhiteSpace>,
    object_fit: Option<ObjectFit>,
    object_position: Option<ObjectPosition>,
}

impl Declarations {
    fn apply(&self, style: &mut ComputedStyle) {
        macro_rules! apply {
            ($field:ident) => {
                if let Some(value) = &self.$field {
                    style.$field = value.clone();
                }
            };
        }

        apply!(display);
        apply!(flex_direction);
        apply!(flex_grow);
        apply!(flex_shrink);
        apply!(width);
        apply!(height);
        apply!(min_width);
        apply!(min_height);
        apply!(max_width);
        apply!(max_height);
        apply!(gap);
        apply!(margin);
        apply!(padding);
        apply!(align_items);
        if let Some(value) = self.align_self {
            style.align_self = Some(value);
        }
        apply!(justify_content);
        apply!(color);
        apply!(background_color);
        apply!(opacity);
        apply!(border_width);
        apply!(border_color);
        apply!(border_radius);
        apply!(overflow);
        apply!(font_family);
        apply!(font_size);
        apply!(font_weight);
        apply!(line_height);
        apply!(text_align);
        apply!(text_overflow);
        apply!(white_space);
        apply!(object_fit);
        apply!(object_position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::WidgetKind;

    fn widget(kind: WidgetKind, id: Option<&str>, classes: &[&str]) -> Widget {
        Widget::new(
            "root".to_string(),
            id.map(str::to_string),
            classes.iter().map(|class| class.to_string()).collect(),
            kind,
        )
    }

    #[test]
    fn selector_specificity_and_order_are_applied() {
        let stylesheet = StyleSheet::parse(
            "test.css",
            r#"
text { color: #111111; }
.metric { color: #222222; font-size: 20px; }
text.metric { color: #333333; }
#cpu { color: #444444; }
#cpu { font-size: 24px; }
"#,
        )
        .unwrap();
        let widget = widget(
            WidgetKind::Text {
                text: crate::Binding::parse("CPU").unwrap(),
            },
            Some("cpu"),
            &["metric"],
        );

        let style = stylesheet.compute(&widget, None);
        assert_eq!(
            style.color,
            Color {
                red: 0x44,
                green: 0x44,
                blue: 0x44,
                alpha: 0xff,
            }
        );
        assert_eq!(style.font_size, 24.0);
    }

    #[test]
    fn inherited_properties_flow_to_children() {
        let stylesheet = StyleSheet::parse(
            "test.css",
            r#"
.parent {
    color: #123456;
    font-family: "Test Font";
    font-size: 22px;
    text-align: center;
}
"#,
        )
        .unwrap();
        let child = widget(
            WidgetKind::Text {
                text: crate::Binding::parse("child").unwrap(),
            },
            None,
            &[],
        );
        let parent = widget(
            WidgetKind::Flex {
                direction: FlexDirection::Row,
                children: vec![child],
            },
            None,
            &["parent"],
        );

        let tree = stylesheet.compute_tree(&parent);
        let child_style = &tree.children[0].style;
        assert_eq!(child_style.color, tree.style.color);
        assert_eq!(child_style.font_family, "Test Font");
        assert_eq!(child_style.font_size, 22.0);
        assert_eq!(child_style.text_align, TextAlign::Center);
        assert_eq!(child_style.background_color, Color::TRANSPARENT);
    }

    #[test]
    fn rejects_unsupported_selector_syntax() {
        let error = StyleSheet::parse("test.css", ".parent text { color: #ffffff; }")
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported selector"), "{error}");
    }
}
