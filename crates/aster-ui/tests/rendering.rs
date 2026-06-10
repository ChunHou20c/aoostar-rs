// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: Copyright (c) 2026 Chunhou Wong

use aster_ui::{Dashboard, Renderer};
use image::{Rgba, RgbaImage};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn font_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fonts/DejaVuSans.ttf")
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
}

fn cjk_font_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fonts/HarmonyOS_Sans_SC_Bold.ttf")
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn text_has_intrinsic_size_and_renders_pixels() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
.root { padding: 4px; background-color: #112233; }
.label { color: #ffffff; font-family: "DejaVu Sans"; font-size: 20px; }
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        format!(
            r##"
[dashboard]
width = 160
height = 48
stylesheet = "dashboard.css"
background = "#000000"
fonts = [{}]

[root]
type = "row"
class = ["root"]

[[root.children]]
type = "text"
class = ["label"]
text = "Aster"
"##,
            toml::Value::String(font_path())
        ),
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let mut renderer = Renderer::new(&dashboard).unwrap();
    let layout = renderer.compute_layout(&dashboard).unwrap();
    let label = layout.root().find("root.children[0]").unwrap();
    assert!(label.width() > 20.0);
    assert!(label.height() >= 20.0);

    let output = renderer.render(&dashboard).unwrap();
    assert_eq!(output.dimensions(), (160, 48));
    assert!(
        output
            .pixels()
            .any(|pixel| *pixel == Rgba([255, 255, 255, 255]))
    );
}

#[test]
fn latin_and_cjk_text_render_with_explicit_fonts() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
.root { background-color: #000000; }
.label { color: #ffffff; font-family: "HarmonyOS Sans SC"; font-size: 20px; }
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        format!(
            r##"
[dashboard]
width = 180
height = 48
stylesheet = "dashboard.css"
fonts = [{}, {}]

[root]
type = "row"
class = ["root"]

[[root.children]]
type = "text"
class = ["label"]
text = "Aster 系统"
"##,
            toml::Value::String(font_path()),
            toml::Value::String(cjk_font_path())
        ),
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let output = Renderer::new(&dashboard)
        .unwrap()
        .render(&dashboard)
        .unwrap();
    let painted = output
        .pixels()
        .filter(|pixel| pixel.0 != [0, 0, 0, 255])
        .count();
    assert!(painted > 100);
}

#[test]
fn image_uses_intrinsic_aspect_ratio() {
    let directory = tempdir().unwrap();
    let mut source = RgbaImage::new(20, 10);
    source.fill(255);
    source.save(directory.path().join("source.png")).unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
.image { width: 40px; align-self: start; object-fit: contain; }
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 80
height = 40
stylesheet = "dashboard.css"

[root]
type = "row"

[[root.children]]
type = "image"
class = ["image"]
source = "source.png"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let mut renderer = Renderer::new(&dashboard).unwrap();
    let layout = renderer.compute_layout(&dashboard).unwrap();
    let image = layout.root().find("root.children[0]").unwrap();
    assert_eq!((image.width(), image.height()), (40.0, 20.0));

    let output = renderer.render(&dashboard).unwrap();
    assert_eq!(output.get_pixel(20, 10), &Rgba([255, 255, 255, 255]));
}

#[test]
fn image_object_fit_modes_are_clipped_to_the_content_box() {
    let directory = tempdir().unwrap();
    let mut source = RgbaImage::new(20, 10);
    source.fill(255);
    source.save(directory.path().join("source.png")).unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
image { width: 40px; height: 40px; flex-shrink: 0; }
.fill { object-fit: fill; }
.contain { object-fit: contain; }
.cover { object-fit: cover; }
.none { object-fit: none; }
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 160
height = 40
stylesheet = "dashboard.css"

[root]
type = "row"

[[root.children]]
type = "image"
class = ["fill"]
source = "source.png"

[[root.children]]
type = "image"
class = ["contain"]
source = "source.png"

[[root.children]]
type = "image"
class = ["cover"]
source = "source.png"

[[root.children]]
type = "image"
class = ["none"]
source = "source.png"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let output = Renderer::new(&dashboard)
        .unwrap()
        .render(&dashboard)
        .unwrap();
    let transparent = Rgba([0, 0, 0, 0]);
    let white = Rgba([255, 255, 255, 255]);
    assert_eq!(output.get_pixel(20, 5), &white);
    assert_eq!(output.get_pixel(60, 5), &transparent);
    assert_eq!(output.get_pixel(60, 20), &white);
    assert_eq!(output.get_pixel(100, 5), &white);
    assert_eq!(output.get_pixel(120, 20), &transparent);
    assert_eq!(output.get_pixel(140, 20), &white);
}

#[test]
fn opacity_and_hidden_overflow_affect_painted_pixels() {
    let directory = tempdir().unwrap();
    let mut source = RgbaImage::new(40, 20);
    source.fill(255);
    source.save(directory.path().join("source.png")).unwrap();
    fs::write(
        directory.path().join("dashboard.css"),
        r#"
.clip {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    overflow: hidden;
    opacity: 0.5;
}
.image {
    width: 40px;
    height: 20px;
    flex-shrink: 0;
    object-fit: fill;
}
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("dashboard.toml"),
        r#"
[dashboard]
width = 40
height = 20
stylesheet = "dashboard.css"

[root]
type = "row"

[[root.children]]
type = "column"
class = ["clip"]

[[root.children.children]]
type = "image"
class = ["image"]
source = "source.png"
"#,
    )
    .unwrap();

    let dashboard = Dashboard::load(directory.path().join("dashboard.toml")).unwrap();
    let output = Renderer::new(&dashboard)
        .unwrap()
        .render(&dashboard)
        .unwrap();
    assert_eq!(output.get_pixel(10, 10), &Rgba([255, 255, 255, 128]));
    assert_eq!(output.get_pixel(25, 10), &Rgba([0, 0, 0, 0]));
}
