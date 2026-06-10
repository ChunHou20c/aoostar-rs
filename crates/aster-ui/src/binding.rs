// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use std::collections::HashMap;
use std::fmt;

pub type ValueMap = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    segments: Vec<Segment>,
}

impl Binding {
    pub(crate) fn parse(input: &str) -> Result<Self, BindingParseError> {
        let mut segments = Vec::new();
        let mut remaining = input;

        while let Some(open) = remaining.find("{{") {
            if open > 0 {
                segments.push(Segment::Literal(remaining[..open].to_string()));
            }
            remaining = &remaining[open + 2..];
            let close = remaining
                .find("}}")
                .ok_or_else(|| BindingParseError("unclosed binding expression".to_string()))?;
            segments.push(Segment::Expression(parse_expression(
                remaining[..close].trim(),
            )?));
            remaining = &remaining[close + 2..];
        }

        if remaining.contains("}}") {
            return Err(BindingParseError(
                "binding expression has an unexpected closing delimiter".to_string(),
            ));
        }
        if !remaining.is_empty() {
            segments.push(Segment::Literal(remaining.to_string()));
        }
        if segments.is_empty() {
            segments.push(Segment::Literal(String::new()));
        }

        Ok(Self { segments })
    }

    pub fn resolve(&self, values: &ValueMap) -> Result<String, BindingResolveError> {
        let mut output = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(value) => output.push_str(value),
                Segment::Expression(expression) => {
                    output.push_str(&expression.resolve(values)?);
                }
            }
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Literal(String),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
struct Expression {
    key: String,
    filters: Vec<Filter>,
}

impl Expression {
    fn resolve(&self, values: &ValueMap) -> Result<String, BindingResolveError> {
        let mut value = values.get(&self.key).cloned();
        for filter in &self.filters {
            match filter {
                Filter::Default(fallback) => {
                    if value.as_deref().is_none_or(str::is_empty) {
                        value = Some(fallback.clone());
                    }
                }
                Filter::Number(precision) => {
                    let Some(raw) = value.as_deref() else {
                        continue;
                    };
                    if raw.is_empty() {
                        continue;
                    }
                    let number = raw.parse::<f64>().map_err(|_| BindingResolveError {
                        key: self.key.clone(),
                        message: format!("expected a number, got {raw:?}"),
                    })?;
                    if !number.is_finite() {
                        return Err(BindingResolveError {
                            key: self.key.clone(),
                            message: format!("expected a finite number, got {raw:?}"),
                        });
                    }
                    value = Some(format!("{number:.precision$}"));
                }
            }
        }
        Ok(value.unwrap_or_default())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Filter {
    Default(String),
    Number(usize),
}

fn parse_expression(input: &str) -> Result<Expression, BindingParseError> {
    if input.is_empty() {
        return Err(BindingParseError(
            "binding expression requires a sensor key".to_string(),
        ));
    }
    let parts = split_filters(input)?;
    let key = parts[0].trim();
    if key.is_empty() {
        return Err(BindingParseError(
            "binding expression requires a sensor key".to_string(),
        ));
    }
    let filters = parts[1..]
        .iter()
        .map(|part| parse_filter(part.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression {
        key: key.to_string(),
        filters,
    })
}

fn split_filters(input: &str) -> Result<Vec<&str>, BindingParseError> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote.is_some() {
            escaped = true;
            continue;
        }
        if matches!(character, '"' | '\'') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            continue;
        }
        if character == '|' && quote.is_none() {
            parts.push(&input[start..index]);
            start = index + 1;
        }
    }
    if quote.is_some() {
        return Err(BindingParseError(
            "binding filter contains an unclosed quoted string".to_string(),
        ));
    }
    parts.push(&input[start..]);
    Ok(parts)
}

fn parse_filter(input: &str) -> Result<Filter, BindingParseError> {
    if let Some(argument) = function_argument(input, "default") {
        return Ok(Filter::Default(parse_quoted_string(argument)?));
    }
    if let Some(argument) = function_argument(input, "number") {
        let precision = argument.trim().parse::<usize>().map_err(|_| {
            BindingParseError(format!(
                "number filter requires a non-negative integer, got {argument:?}"
            ))
        })?;
        if precision > 10 {
            return Err(BindingParseError(
                "number filter precision must be between 0 and 10".to_string(),
            ));
        }
        return Ok(Filter::Number(precision));
    }
    Err(BindingParseError(format!(
        "unsupported binding filter {input:?}; expected default(\"value\") or number(digits)"
    )))
}

fn function_argument<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    input
        .strip_prefix(name)
        .and_then(|suffix| suffix.strip_prefix('('))
        .and_then(|suffix| suffix.strip_suffix(')'))
}

fn parse_quoted_string(input: &str) -> Result<String, BindingParseError> {
    let input = input.trim();
    let Some(quote) = input
        .chars()
        .next()
        .filter(|value| matches!(value, '"' | '\''))
    else {
        return Err(BindingParseError(
            "default filter requires a quoted string".to_string(),
        ));
    };
    if input.len() < 2 || !input.ends_with(quote) {
        return Err(BindingParseError(
            "default filter requires a quoted string".to_string(),
        ));
    }
    let inner = &input[quote.len_utf8()..input.len() - quote.len_utf8()];
    let mut output = String::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            let escaped = characters.next().ok_or_else(|| {
                BindingParseError("default string ends with an escape character".to_string())
            })?;
            match escaped {
                '\\' => output.push('\\'),
                '"' => output.push('"'),
                '\'' => output.push('\''),
                'n' => output.push('\n'),
                't' => output.push('\t'),
                _ => {
                    return Err(BindingParseError(format!(
                        "unsupported escape sequence \\{escaped}"
                    )));
                }
            }
        } else {
            output.push(character);
        }
    }
    Ok(output)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BindingParseError(String);

impl fmt::Display for BindingParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingResolveError {
    key: String,
    message: String,
}

impl fmt::Display for BindingResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "sensor {:?}: {}", self.key, self.message)
    }
}

impl std::error::Error for BindingResolveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_multiple_bindings_and_filters() {
        let binding = Binding::parse(
            r#"CPU {{ cpu | number(1) }}%, host {{ hostname | default("unknown") }}"#,
        )
        .unwrap();
        let values = ValueMap::from([("cpu".to_string(), "47.66".to_string())]);

        assert_eq!(binding.resolve(&values).unwrap(), "CPU 47.7%, host unknown");
    }

    #[test]
    fn missing_values_resolve_to_empty_strings() {
        let binding = Binding::parse("{{ missing }}").unwrap();
        assert_eq!(binding.resolve(&ValueMap::new()).unwrap(), "");
    }

    #[test]
    fn rejects_invalid_syntax_at_parse_time() {
        assert!(Binding::parse("{{ value | unknown }}").is_err());
        assert!(Binding::parse("{{ value").is_err());
        assert!(Binding::parse("{{ value | number(-1) }}").is_err());
    }

    #[test]
    fn rejects_non_numeric_present_values() {
        let binding = Binding::parse("{{ value | number(0) }}").unwrap();
        let values = ValueMap::from([("value".to_string(), "invalid".to_string())]);
        assert!(binding.resolve(&values).is_err());
    }
}
