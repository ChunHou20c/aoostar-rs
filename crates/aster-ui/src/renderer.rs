// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use crate::config::Dashboard;
use crate::error::DashboardError;
use crate::layout::{LayoutNode, LayoutTree};
use crate::style::{
    Color, ComputedStyle, ObjectFit, ObjectPosition, Overflow, TextAlign, WhiteSpace,
};
use crate::widget::{Widget, WidgetKind};
use cosmic_text::{
    Align as CosmicAlign, Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics,
    Shaping, SwashCache, Weight, Wrap,
};
use image::{RgbaImage, imageops::FilterType};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use taffy::prelude::{AvailableSpace, Size};
use tiny_skia::{
    FillRule, Paint, Path as SkiaPath, PathBuilder, Pixmap, PremultipliedColorU8, Rect, Stroke,
    Transform,
};

pub struct Renderer {
    assets: AssetCache,
}

impl Renderer {
    pub fn new(dashboard: &Dashboard) -> Result<Self, DashboardError> {
        Ok(Self {
            assets: AssetCache::new(dashboard.options().fonts())?,
        })
    }

    pub fn compute_layout(&mut self, dashboard: &Dashboard) -> Result<LayoutTree, DashboardError> {
        LayoutTree::compute_with_assets(
            dashboard.root(),
            dashboard.stylesheet(),
            dashboard.options().width(),
            dashboard.options().height(),
            &mut self.assets,
        )
    }

    pub fn render(&mut self, dashboard: &Dashboard) -> Result<RgbaImage, DashboardError> {
        let layout = self.compute_layout(dashboard)?;
        let mut pixmap = Pixmap::new(dashboard.options().width(), dashboard.options().height())
            .ok_or_else(|| DashboardError::render("dashboard dimensions are too large"))?;

        if let Some(background) = dashboard.options().background() {
            pixmap.fill(to_skia_color(parse_hex_color(background)?));
        }

        let clip = ClipRect::new(
            0,
            0,
            dashboard.options().width() as i32,
            dashboard.options().height() as i32,
        );
        self.paint_node(&mut pixmap, dashboard.root(), layout.root(), clip, 1.0)?;

        let mut output = RgbaImage::new(pixmap.width(), pixmap.height());
        for (target, source) in output.pixels_mut().zip(pixmap.pixels()) {
            let color = source.demultiply();
            *target = image::Rgba([color.red(), color.green(), color.blue(), color.alpha()]);
        }
        Ok(output)
    }

    fn paint_node(
        &mut self,
        pixmap: &mut Pixmap,
        widget: &Widget,
        layout: &LayoutNode,
        parent_clip: ClipRect,
        parent_opacity: f32,
    ) -> Result<(), DashboardError> {
        if layout.width() <= 0.0 || layout.height() <= 0.0 {
            return Ok(());
        }

        let opacity = (parent_opacity * layout.style().opacity).clamp(0.0, 1.0);
        paint_box(pixmap, layout, opacity, parent_clip);
        let own_clip = ClipRect::from_layout(layout);
        let child_clip = if layout.style().overflow == Overflow::Hidden {
            parent_clip.intersect(own_clip)
        } else {
            parent_clip
        };

        match widget.kind() {
            WidgetKind::Text { text } => {
                self.assets
                    .paint_text(pixmap, text, layout, opacity, child_clip);
            }
            WidgetKind::Image { source } => {
                let image_clip = child_clip.intersect(ClipRect::from_content(layout));
                self.assets
                    .paint_image(pixmap, source, layout, opacity, image_clip)?;
            }
            _ => {}
        }

        for (child, child_layout) in widget.children().iter().zip(layout.children()) {
            self.paint_node(pixmap, child, child_layout, child_clip, opacity)?;
        }
        Ok(())
    }
}

pub(crate) struct AssetCache {
    font_system: FontSystem,
    swash_cache: SwashCache,
    images: HashMap<PathBuf, RgbaImage>,
}

impl AssetCache {
    fn new(fonts: &[PathBuf]) -> Result<Self, DashboardError> {
        let sources = fonts
            .iter()
            .map(|path| {
                fs::read(path)
                    .map(|bytes| fontdb::Source::Binary(Arc::new(bytes)))
                    .map_err(|error| DashboardError::asset(path, error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            font_system: FontSystem::new_with_fonts(sources),
            swash_cache: SwashCache::new(),
            images: HashMap::new(),
        })
    }

    pub(crate) fn measure_text(
        &mut self,
        text: &str,
        style: &ComputedStyle,
        known: Size<Option<f32>>,
        available: Size<AvailableSpace>,
    ) -> Result<Size<f32>, DashboardError> {
        if let Size {
            width: Some(width),
            height: Some(height),
        } = known
        {
            return Ok(Size { width, height });
        }
        let width = known.width.or_else(|| match available.width {
            AvailableSpace::Definite(width) if style.white_space != WhiteSpace::NoWrap => {
                Some(width)
            }
            _ => None,
        });
        let buffer = self.shape_text(text, style, width, None);
        let measured = text_bounds(&buffer);
        Ok(Size {
            width: known.width.unwrap_or(measured.width),
            height: known.height.unwrap_or(measured.height),
        })
    }

    pub(crate) fn measure_image(
        &mut self,
        source: &Path,
        known: Size<Option<f32>>,
    ) -> Result<Size<f32>, DashboardError> {
        let image = self.image(source)?;
        let natural = Size {
            width: image.width() as f32,
            height: image.height() as f32,
        };
        Ok(match known {
            Size {
                width: Some(width),
                height: Some(height),
            } => Size { width, height },
            Size {
                width: Some(width),
                height: None,
            } => Size {
                width,
                height: width * natural.height / natural.width,
            },
            Size {
                width: None,
                height: Some(height),
            } => Size {
                width: height * natural.width / natural.height,
                height,
            },
            _ => natural,
        })
    }

    fn shape_text(
        &mut self,
        text: &str,
        style: &ComputedStyle,
        width: Option<f32>,
        height: Option<f32>,
    ) -> Buffer {
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(style.font_size, style.font_size * style.line_height),
        );
        buffer.set_wrap(
            &mut self.font_system,
            if style.white_space == WhiteSpace::NoWrap {
                Wrap::None
            } else {
                Wrap::WordOrGlyph
            },
        );
        buffer.set_size(&mut self.font_system, width, height);
        let attrs = Attrs::new()
            .family(Family::Name(&style.font_family))
            .weight(Weight(style.font_weight));
        buffer.set_text(
            &mut self.font_system,
            text,
            &attrs,
            Shaping::Advanced,
            Some(match style.text_align {
                TextAlign::Start => CosmicAlign::Left,
                TextAlign::Center => CosmicAlign::Center,
                TextAlign::End => CosmicAlign::Right,
            }),
        );
        buffer.shape_until_scroll(&mut self.font_system, true);
        buffer
    }

    fn paint_text(
        &mut self,
        pixmap: &mut Pixmap,
        text: &str,
        layout: &LayoutNode,
        opacity: f32,
        clip: ClipRect,
    ) {
        let style = layout.style();
        let buffer = self.shape_text(
            text,
            style,
            Some(layout.content_width()),
            Some(layout.content_height()),
        );
        let color = with_opacity(style.color, opacity);
        let origin_x = layout.content_x().round() as i32;
        let origin_y = layout.content_y().round() as i32;
        let width = pixmap.width();
        let pixels = pixmap.pixels_mut();
        buffer.draw(
            &mut self.font_system,
            &mut self.swash_cache,
            CosmicColor::rgba(color.red, color.green, color.blue, color.alpha),
            |x, y, glyph_width, glyph_height, glyph_color| {
                let [red, green, blue, alpha] = glyph_color.as_rgba();
                for row in 0..glyph_height as i32 {
                    for column in 0..glyph_width as i32 {
                        let px = origin_x + x + column;
                        let py = origin_y + y + row;
                        if clip.contains(px, py) {
                            blend_pixel(pixels, width, px, py, [red, green, blue, alpha]);
                        }
                    }
                }
            },
        );
    }

    fn paint_image(
        &mut self,
        pixmap: &mut Pixmap,
        source: &Path,
        layout: &LayoutNode,
        opacity: f32,
        clip: ClipRect,
    ) -> Result<(), DashboardError> {
        let image = self.image(source)?.clone();
        let box_width = layout.content_width().max(0.0);
        let box_height = layout.content_height().max(0.0);
        if box_width == 0.0 || box_height == 0.0 {
            return Ok(());
        }
        let natural_width = image.width() as f32;
        let natural_height = image.height() as f32;
        let (width, height) = match layout.style().object_fit {
            ObjectFit::Fill => (box_width, box_height),
            ObjectFit::Contain => {
                let scale = (box_width / natural_width).min(box_height / natural_height);
                (natural_width * scale, natural_height * scale)
            }
            ObjectFit::Cover => {
                let scale = (box_width / natural_width).max(box_height / natural_height);
                (natural_width * scale, natural_height * scale)
            }
            ObjectFit::None => (natural_width, natural_height),
        };
        let offset = |space: f32| match layout.style().object_position {
            ObjectPosition::Start => 0.0,
            ObjectPosition::Center => space / 2.0,
            ObjectPosition::End => space,
        };
        let x = (layout.content_x() + offset(box_width - width)).round() as i32;
        let y = (layout.content_y() + offset(box_height - height)).round() as i32;
        let resized = image::imageops::resize(
            &image,
            width.max(1.0).round() as u32,
            height.max(1.0).round() as u32,
            FilterType::Triangle,
        );
        let canvas_width = pixmap.width();
        let pixels = pixmap.pixels_mut();
        for (source_x, source_y, pixel) in resized.enumerate_pixels() {
            let target_x = x + source_x as i32;
            let target_y = y + source_y as i32;
            if clip.contains(target_x, target_y) {
                let mut rgba = pixel.0;
                rgba[3] = (rgba[3] as f32 * opacity).round() as u8;
                blend_pixel(pixels, canvas_width, target_x, target_y, rgba);
            }
        }
        Ok(())
    }

    fn image(&mut self, source: &Path) -> Result<&RgbaImage, DashboardError> {
        if !self.images.contains_key(source) {
            let image = image::open(source)
                .map_err(|error| DashboardError::asset(source, error.to_string()))?
                .to_rgba8();
            self.images.insert(source.to_path_buf(), image);
        }
        Ok(self.images.get(source).expect("image inserted above"))
    }
}

fn text_bounds(buffer: &Buffer) -> Size<f32> {
    let mut width: f32 = 0.0;
    let mut height: f32 = 0.0;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        height = height.max(run.line_top + run.line_height);
    }
    Size { width, height }
}

fn paint_box(pixmap: &mut Pixmap, layout: &LayoutNode, opacity: f32, clip: ClipRect) {
    let style = layout.style();
    let background = with_opacity(style.background_color, opacity);
    let border = with_opacity(style.border_color, opacity);
    if style.border_radius > 0.0 {
        if let Some(path) = rounded_rect_path(
            layout.x(),
            layout.y(),
            layout.width(),
            layout.height(),
            style.border_radius,
        ) {
            fill_path(pixmap, &path, background, clip);
        }
        if style.border_width > 0.0 {
            let inset = style.border_width / 2.0;
            if let Some(path) = rounded_rect_path(
                layout.x() + inset,
                layout.y() + inset,
                layout.width() - style.border_width,
                layout.height() - style.border_width,
                (style.border_radius - inset).max(0.0),
            ) {
                stroke_path(pixmap, &path, border, style.border_width, clip);
            }
        }
        return;
    }

    fill_rect(
        pixmap,
        layout.x(),
        layout.y(),
        layout.width(),
        layout.height(),
        background,
        clip,
    );
    let width = style.border_width.min(layout.width() / 2.0);
    let height = style.border_width.min(layout.height() / 2.0);
    if width > 0.0 || height > 0.0 {
        fill_rect(
            pixmap,
            layout.x(),
            layout.y(),
            layout.width(),
            height,
            border,
            clip,
        );
        fill_rect(
            pixmap,
            layout.x(),
            layout.y() + layout.height() - height,
            layout.width(),
            height,
            border,
            clip,
        );
        fill_rect(
            pixmap,
            layout.x(),
            layout.y() + height,
            width,
            layout.height() - height * 2.0,
            border,
            clip,
        );
        fill_rect(
            pixmap,
            layout.x() + layout.width() - width,
            layout.y() + height,
            width,
            layout.height() - height * 2.0,
            border,
            clip,
        );
    }
}

fn rounded_rect_path(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Option<SkiaPath> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let radius = radius.min(width / 2.0).min(height / 2.0);
    let control = radius * 0.552_284_8;
    let right = x + width;
    let bottom = y + height;
    let mut path = PathBuilder::new();
    path.move_to(x + radius, y);
    path.line_to(right - radius, y);
    path.cubic_to(
        right - radius + control,
        y,
        right,
        y + radius - control,
        right,
        y + radius,
    );
    path.line_to(right, bottom - radius);
    path.cubic_to(
        right,
        bottom - radius + control,
        right - radius + control,
        bottom,
        right - radius,
        bottom,
    );
    path.line_to(x + radius, bottom);
    path.cubic_to(
        x + radius - control,
        bottom,
        x,
        bottom - radius + control,
        x,
        bottom - radius,
    );
    path.line_to(x, y + radius);
    path.cubic_to(
        x,
        y + radius - control,
        x + radius - control,
        y,
        x + radius,
        y,
    );
    path.close();
    path.finish()
}

fn fill_path(pixmap: &mut Pixmap, path: &SkiaPath, color: Color, clip: ClipRect) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);
    let mask = clip_mask(pixmap, clip);
    pixmap.fill_path(
        path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        mask.as_ref(),
    );
}

fn stroke_path(pixmap: &mut Pixmap, path: &SkiaPath, color: Color, width: f32, clip: ClipRect) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);
    let stroke = Stroke {
        width,
        ..Stroke::default()
    };
    let mask = clip_mask(pixmap, clip);
    pixmap.stroke_path(path, &paint, &stroke, Transform::identity(), mask.as_ref());
}

fn clip_mask(pixmap: &Pixmap, clip: ClipRect) -> Option<tiny_skia::Mask> {
    let mut mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height())?;
    let rect = Rect::from_xywh(
        clip.left as f32,
        clip.top as f32,
        (clip.right - clip.left).max(0) as f32,
        (clip.bottom - clip.top).max(0) as f32,
    )?;
    let path = PathBuilder::from_rect(rect);
    mask.fill_path(&path, FillRule::Winding, false, Transform::identity());
    Some(mask)
}

fn fill_rect(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
    clip: ClipRect,
) {
    let left = x.max(clip.left as f32);
    let top = y.max(clip.top as f32);
    let right = (x + width).min(clip.right as f32);
    let bottom = (y + height).min(clip.bottom as f32);
    let Some(rect) = Rect::from_xywh(left, top, right - left, bottom - top) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn blend_pixel(
    pixels: &mut [PremultipliedColorU8],
    canvas_width: u32,
    x: i32,
    y: i32,
    source: [u8; 4],
) {
    let index = y as usize * canvas_width as usize + x as usize;
    let destination = pixels[index];
    let source_alpha = source[3] as u32;
    let inverse_alpha = 255 - source_alpha;
    let output_alpha = source_alpha + destination.alpha() as u32 * inverse_alpha / 255;
    let channel = |source_channel: u8, destination_channel: u8| {
        (source_channel as u32 * source_alpha / 255
            + destination_channel as u32 * inverse_alpha / 255)
            .min(output_alpha) as u8
    };
    pixels[index] = PremultipliedColorU8::from_rgba(
        channel(source[0], destination.red()),
        channel(source[1], destination.green()),
        channel(source[2], destination.blue()),
        output_alpha as u8,
    )
    .expect("source-over produces valid premultiplied color");
}

fn with_opacity(mut color: Color, opacity: f32) -> Color {
    color.alpha = (color.alpha as f32 * opacity).round() as u8;
    color
}

fn to_skia_color(color: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(color.red, color.green, color.blue, color.alpha)
}

fn parse_hex_color(value: &str) -> Result<Color, DashboardError> {
    let value = value
        .strip_prefix('#')
        .ok_or_else(|| DashboardError::render(format!("invalid color {value:?}")))?;
    let parse = |range| {
        u8::from_str_radix(&value[range], 16)
            .map_err(|_| DashboardError::render(format!("invalid color #{value}")))
    };
    Ok(Color {
        red: parse(0..2)?,
        green: parse(2..4)?,
        blue: parse(4..6)?,
        alpha: if value.len() == 8 { parse(6..8)? } else { 255 },
    })
}

#[derive(Clone, Copy)]
struct ClipRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl ClipRect {
    fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    fn from_layout(layout: &LayoutNode) -> Self {
        Self::new(
            layout.x().floor() as i32,
            layout.y().floor() as i32,
            (layout.x() + layout.width()).ceil() as i32,
            (layout.y() + layout.height()).ceil() as i32,
        )
    }

    fn from_content(layout: &LayoutNode) -> Self {
        Self::new(
            layout.content_x().floor() as i32,
            layout.content_y().floor() as i32,
            (layout.content_x() + layout.content_width()).ceil() as i32,
            (layout.content_y() + layout.content_height()).ceil() as i32,
        )
    }

    fn intersect(self, other: Self) -> Self {
        Self::new(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        )
    }

    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}
