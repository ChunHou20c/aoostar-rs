// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use super::{
    Align, Color, Declarations, Display, Edges, FlexDirectionStyle, JustifyContent, Length,
    ObjectFit, ObjectPosition, Overflow, Rule, Selector, TextAlign, TextOverflow, WhiteSpace,
};
use crate::error::DashboardError;
use std::path::Path;

pub(super) fn parse_stylesheet(path: &Path, contents: &str) -> Result<Vec<Rule>, DashboardError> {
    let contents = strip_comments(path, contents)?;
    let mut rules = Vec::new();
    let mut rest = contents.as_str();

    while !rest.trim().is_empty() {
        let Some(open) = rest.find('{') else {
            return Err(error(path, "expected '{' after selector"));
        };
        let selector_source = rest[..open].trim();
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            return Err(error(path, "expected '}' after declarations"));
        };
        if after_open[..close].contains('{') {
            return Err(error(path, "nested CSS blocks are not supported"));
        }

        let selector = parse_selector(path, selector_source)?;
        let declarations = parse_declarations(path, &after_open[..close])?;
        rules.push(Rule {
            selector,
            declarations,
            order: rules.len(),
        });
        rest = &after_open[close + 1..];
    }

    Ok(rules)
}

fn strip_comments(path: &Path, contents: &str) -> Result<String, DashboardError> {
    let mut result = String::with_capacity(contents.len());
    let mut rest = contents;

    while let Some(start) = rest.find("/*") {
        result.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("*/") else {
            return Err(error(path, "unterminated CSS comment"));
        };
        rest = &after_start[end + 2..];
    }
    result.push_str(rest);
    Ok(result)
}

fn parse_selector(path: &Path, source: &str) -> Result<Selector, DashboardError> {
    if source.is_empty()
        || source.contains(|character: char| character.is_whitespace())
        || source.contains(',')
        || source.contains('>')
        || source.contains('[')
        || source.contains(':')
    {
        return Err(error(
            path,
            format!("unsupported selector {source:?}; expected type, .class, type.class, or #id"),
        ));
    }

    if let Some(id) = source.strip_prefix('#') {
        validate_identifier(path, id, "ID selector")?;
        return Ok(Selector::Id(id.to_string()));
    }
    if let Some(class) = source.strip_prefix('.') {
        validate_identifier(path, class, "class selector")?;
        return Ok(Selector::Class(class.to_string()));
    }
    if let Some((widget_type, class)) = source.split_once('.') {
        validate_widget_type(path, widget_type)?;
        validate_identifier(path, class, "class selector")?;
        if class.contains('.') {
            return Err(error(path, "selectors may contain only one class"));
        }
        return Ok(Selector::TypeClass {
            widget_type: widget_type.to_string(),
            class: class.to_string(),
        });
    }

    validate_widget_type(path, source)?;
    Ok(Selector::Type(source.to_string()))
}

fn parse_declarations(path: &Path, source: &str) -> Result<Declarations, DashboardError> {
    let mut declarations = Declarations::default();

    for declaration in source.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        let Some((property, value)) = declaration.split_once(':') else {
            return Err(error(
                path,
                format!("invalid declaration {declaration:?}; expected property: value"),
            ));
        };
        let property = property.trim();
        let value = value.trim();
        if value.is_empty() {
            return Err(error(path, format!("{property} requires a value")));
        }

        match property {
            "display" => declarations.display = Some(parse_display(path, value)?),
            "flex-direction" => {
                declarations.flex_direction = Some(parse_flex_direction(path, value)?)
            }
            "flex-grow" => {
                declarations.flex_grow = Some(parse_non_negative(path, property, value)?)
            }
            "flex-shrink" => {
                declarations.flex_shrink = Some(parse_non_negative(path, property, value)?)
            }
            "width" => declarations.width = Some(parse_length(path, property, value, true)?),
            "height" => declarations.height = Some(parse_length(path, property, value, true)?),
            "min-width" => {
                declarations.min_width = Some(parse_length(path, property, value, true)?)
            }
            "min-height" => {
                declarations.min_height = Some(parse_length(path, property, value, true)?)
            }
            "max-width" => {
                declarations.max_width = Some(parse_length(path, property, value, true)?)
            }
            "max-height" => {
                declarations.max_height = Some(parse_length(path, property, value, true)?)
            }
            "gap" => declarations.gap = Some(parse_length(path, property, value, false)?),
            "margin" => {
                declarations.margin = Some(Edges::all(parse_length(path, property, value, true)?))
            }
            "padding" => {
                declarations.padding = Some(Edges::all(parse_length(path, property, value, false)?))
            }
            "align-items" => declarations.align_items = Some(parse_align(path, value)?),
            "align-self" => declarations.align_self = Some(parse_align(path, value)?),
            "justify-content" => declarations.justify_content = Some(parse_justify(path, value)?),
            "color" => declarations.color = Some(parse_color(path, property, value)?),
            "background-color" => {
                declarations.background_color = Some(parse_color(path, property, value)?)
            }
            "opacity" => declarations.opacity = Some(parse_unit_interval(path, property, value)?),
            "border-width" => declarations.border_width = Some(parse_px(path, property, value)?),
            "border-color" => declarations.border_color = Some(parse_color(path, property, value)?),
            "border-radius" => declarations.border_radius = Some(parse_px(path, property, value)?),
            "overflow" => declarations.overflow = Some(parse_overflow(path, value)?),
            "font-family" => declarations.font_family = Some(parse_font_family(path, value)?),
            "font-size" => declarations.font_size = Some(parse_positive_px(path, property, value)?),
            "font-weight" => declarations.font_weight = Some(parse_font_weight(path, value)?),
            "line-height" => {
                declarations.line_height = Some(parse_positive_number(path, property, value)?)
            }
            "text-align" => declarations.text_align = Some(parse_text_align(path, value)?),
            "text-overflow" => declarations.text_overflow = Some(parse_text_overflow(path, value)?),
            "white-space" => declarations.white_space = Some(parse_white_space(path, value)?),
            "object-fit" => declarations.object_fit = Some(parse_object_fit(path, value)?),
            "object-position" => {
                declarations.object_position = Some(parse_object_position(path, value)?)
            }
            _ => return Err(error(path, format!("unsupported property {property:?}"))),
        }
    }

    Ok(declarations)
}

fn parse_display(path: &Path, value: &str) -> Result<Display, DashboardError> {
    match value {
        "flex" => Ok(Display::Flex),
        "stack" => Ok(Display::Stack),
        "none" => Ok(Display::None),
        _ => Err(invalid_value(path, "display", value)),
    }
}

fn parse_flex_direction(path: &Path, value: &str) -> Result<FlexDirectionStyle, DashboardError> {
    match value {
        "row" => Ok(FlexDirectionStyle::Row),
        "column" => Ok(FlexDirectionStyle::Column),
        _ => Err(invalid_value(path, "flex-direction", value)),
    }
}

fn parse_align(path: &Path, value: &str) -> Result<Align, DashboardError> {
    match value {
        "start" | "flex-start" => Ok(Align::Start),
        "end" | "flex-end" => Ok(Align::End),
        "center" => Ok(Align::Center),
        "stretch" => Ok(Align::Stretch),
        _ => Err(invalid_value(path, "alignment", value)),
    }
}

fn parse_justify(path: &Path, value: &str) -> Result<JustifyContent, DashboardError> {
    match value {
        "start" | "flex-start" => Ok(JustifyContent::Start),
        "end" | "flex-end" => Ok(JustifyContent::End),
        "center" => Ok(JustifyContent::Center),
        "space-between" => Ok(JustifyContent::SpaceBetween),
        "space-around" => Ok(JustifyContent::SpaceAround),
        "space-evenly" => Ok(JustifyContent::SpaceEvenly),
        _ => Err(invalid_value(path, "justify-content", value)),
    }
}

fn parse_overflow(path: &Path, value: &str) -> Result<Overflow, DashboardError> {
    match value {
        "visible" => Ok(Overflow::Visible),
        "hidden" => Ok(Overflow::Hidden),
        _ => Err(invalid_value(path, "overflow", value)),
    }
}

fn parse_text_align(path: &Path, value: &str) -> Result<TextAlign, DashboardError> {
    match value {
        "left" | "start" => Ok(TextAlign::Start),
        "center" => Ok(TextAlign::Center),
        "right" | "end" => Ok(TextAlign::End),
        _ => Err(invalid_value(path, "text-align", value)),
    }
}

fn parse_text_overflow(path: &Path, value: &str) -> Result<TextOverflow, DashboardError> {
    match value {
        "clip" => Ok(TextOverflow::Clip),
        "ellipsis" => Ok(TextOverflow::Ellipsis),
        _ => Err(invalid_value(path, "text-overflow", value)),
    }
}

fn parse_white_space(path: &Path, value: &str) -> Result<WhiteSpace, DashboardError> {
    match value {
        "normal" => Ok(WhiteSpace::Normal),
        "nowrap" => Ok(WhiteSpace::NoWrap),
        _ => Err(invalid_value(path, "white-space", value)),
    }
}

fn parse_object_fit(path: &Path, value: &str) -> Result<ObjectFit, DashboardError> {
    match value {
        "fill" => Ok(ObjectFit::Fill),
        "contain" => Ok(ObjectFit::Contain),
        "cover" => Ok(ObjectFit::Cover),
        "none" => Ok(ObjectFit::None),
        _ => Err(invalid_value(path, "object-fit", value)),
    }
}

fn parse_object_position(path: &Path, value: &str) -> Result<ObjectPosition, DashboardError> {
    match value {
        "left" | "top" | "start" => Ok(ObjectPosition::Start),
        "center" => Ok(ObjectPosition::Center),
        "right" | "bottom" | "end" => Ok(ObjectPosition::End),
        _ => Err(invalid_value(path, "object-position", value)),
    }
}

fn parse_font_family(path: &Path, value: &str) -> Result<String, DashboardError> {
    let value = value.trim();
    let is_quoted = value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')));
    let unquoted = if is_quoted {
        &value[1..value.len() - 1]
    } else {
        value
    };
    if unquoted.is_empty() || unquoted.contains(',') {
        return Err(error(
            path,
            "font-family requires one non-empty family name",
        ));
    }
    Ok(unquoted.to_string())
}

fn parse_font_weight(path: &Path, value: &str) -> Result<u16, DashboardError> {
    let weight = match value {
        "normal" => 400,
        "bold" => 700,
        _ => value
            .parse::<u16>()
            .map_err(|_| invalid_value(path, "font-weight", value))?,
    };
    if (100..=900).contains(&weight) && weight % 100 == 0 {
        Ok(weight)
    } else {
        Err(invalid_value(path, "font-weight", value))
    }
}

fn parse_length(
    path: &Path,
    property: &str,
    value: &str,
    allow_auto: bool,
) -> Result<Length, DashboardError> {
    if allow_auto && value == "auto" {
        return Ok(Length::Auto);
    }
    if value == "0" {
        return Ok(Length::Px(0.0));
    }
    if let Some(value) = value.strip_suffix("px") {
        return Ok(Length::Px(parse_non_negative(path, property, value)?));
    }
    if let Some(value) = value.strip_suffix('%') {
        return Ok(Length::Percent(
            parse_non_negative(path, property, value)? / 100.0,
        ));
    }
    Err(error(
        path,
        format!(
            "{property} requires px, %, zero{}",
            if allow_auto { ", or auto" } else { "" }
        ),
    ))
}

fn parse_px(path: &Path, property: &str, value: &str) -> Result<f32, DashboardError> {
    match parse_length(path, property, value, false)? {
        Length::Px(value) => Ok(value),
        Length::Percent(_) | Length::Auto => Err(error(
            path,
            format!("{property} requires a non-negative pixel value"),
        )),
    }
}

fn parse_positive_px(path: &Path, property: &str, value: &str) -> Result<f32, DashboardError> {
    let value = parse_px(path, property, value)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(error(path, format!("{property} must be greater than zero")))
    }
}

fn parse_non_negative(path: &Path, property: &str, value: &str) -> Result<f32, DashboardError> {
    let value = value
        .trim()
        .parse::<f32>()
        .map_err(|_| invalid_value(path, property, value))?;
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(error(
            path,
            format!("{property} must be a finite non-negative number"),
        ))
    }
}

fn parse_positive_number(path: &Path, property: &str, value: &str) -> Result<f32, DashboardError> {
    let value = parse_non_negative(path, property, value)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(error(path, format!("{property} must be greater than zero")))
    }
}

fn parse_unit_interval(path: &Path, property: &str, value: &str) -> Result<f32, DashboardError> {
    let value = parse_non_negative(path, property, value)?;
    if value <= 1.0 {
        Ok(value)
    } else {
        Err(error(path, format!("{property} must be between 0 and 1")))
    }
}

fn parse_color(path: &Path, property: &str, value: &str) -> Result<Color, DashboardError> {
    if value == "transparent" {
        return Ok(Color::TRANSPARENT);
    }
    let Some(hex) = value.strip_prefix('#') else {
        return Err(error(
            path,
            format!("{property} requires #RRGGBB, #RRGGBBAA, or transparent"),
        ));
    };
    if !matches!(hex.len(), 6 | 8) || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(error(path, format!("invalid {property} color {value:?}")));
    }
    let component = |range| u8::from_str_radix(&hex[range], 16).expect("validated hex color");
    Ok(Color {
        red: component(0..2),
        green: component(2..4),
        blue: component(4..6),
        alpha: if hex.len() == 8 { component(6..8) } else { 255 },
    })
}

fn validate_widget_type(path: &Path, value: &str) -> Result<(), DashboardError> {
    if matches!(
        value,
        "row" | "column" | "stack" | "text" | "image" | "spacer" | "progress"
    ) {
        Ok(())
    } else {
        Err(error(
            path,
            format!("unknown widget type selector {value:?}"),
        ))
    }
}

fn validate_identifier(path: &Path, value: &str, label: &str) -> Result<(), DashboardError> {
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        Ok(())
    } else {
        Err(error(path, format!("invalid {label} {value:?}")))
    }
}

fn invalid_value(path: &Path, property: &str, value: &str) -> DashboardError {
    error(path, format!("invalid {property} value {value:?}"))
}

fn error(path: &Path, message: impl Into<String>) -> DashboardError {
    DashboardError::stylesheet(path, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_initial_properties() {
        let rules = parse_stylesheet(
            Path::new("test.css"),
            r#"
.all {
    display: flex;
    flex-direction: column;
    flex-grow: 1;
    flex-shrink: 0;
    width: 50%;
    height: 20px;
    min-width: 0;
    min-height: auto;
    max-width: 100%;
    max-height: 200px;
    gap: 4px;
    margin: auto;
    padding: 8px;
    align-items: center;
    align-self: end;
    justify-content: space-between;
    color: #112233;
    background-color: #44556677;
    opacity: 0.5;
    border-width: 2px;
    border-color: transparent;
    border-radius: 6px;
    overflow: hidden;
    font-family: "DejaVu Sans";
    font-size: 18px;
    font-weight: 700;
    line-height: 1.4;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
    object-fit: cover;
    object-position: center;
}
"#,
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn rejects_unknown_properties() {
        let error = parse_stylesheet(Path::new("test.css"), "text { magic: yes; }")
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsupported property"), "{error}");
    }

    #[test]
    fn rejects_unsupported_units() {
        let error = parse_stylesheet(Path::new("test.css"), "text { width: 10em; }")
            .unwrap_err()
            .to_string();
        assert!(error.contains("requires px"), "{error}");
    }
}
