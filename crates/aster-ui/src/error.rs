// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DashboardError {
    #[error("failed to read dashboard {path:?}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse dashboard {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to parse stylesheet {path:?}: {message}")]
    Stylesheet { path: PathBuf, message: String },

    #[error("invalid dashboard {path:?}: {message}")]
    Validation { path: PathBuf, message: String },

    #[error("failed to compute dashboard layout: {message}")]
    Layout { message: String },

    #[error("failed to load asset {path:?}: {message}")]
    Asset { path: PathBuf, message: String },

    #[error("failed to render dashboard: {message}")]
    Render { message: String },
}

impl DashboardError {
    pub(crate) fn validation(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Validation {
            path: path.into(),
            message: message.into(),
        }
    }

    pub(crate) fn stylesheet(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Stylesheet {
            path: path.into(),
            message: message.into(),
        }
    }

    pub(crate) fn layout(message: impl Into<String>) -> Self {
        Self::Layout {
            message: message.into(),
        }
    }

    pub(crate) fn asset(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Asset {
            path: path.into(),
            message: message.into(),
        }
    }

    pub(crate) fn render(message: impl Into<String>) -> Self {
        Self::Render {
            message: message.into(),
        }
    }
}
