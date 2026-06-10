// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

#![forbid(non_ascii_idents)]
#![deny(unsafe_code)]

mod config;
mod error;
mod layout;
mod style;
mod widget;

pub use config::{Dashboard, DashboardOptions};
pub use error::DashboardError;
pub use layout::{LayoutNode, LayoutTree};
pub use style::{
    Align, Color, ComputedStyle, Display, Edges, FlexDirectionStyle, JustifyContent, Length,
    ObjectFit, ObjectPosition, Overflow, StyleSheet, StyledNode, TextAlign, TextOverflow,
    WhiteSpace,
};
pub use widget::{FlexDirection, ProgressOrientation, Widget, WidgetKind};
