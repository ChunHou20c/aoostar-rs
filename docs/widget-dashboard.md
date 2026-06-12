# Widget Dashboards

Widget dashboards are the planned headless layout mode for `asterctl`. The
`aster-ui` crate currently provides configuration validation, strict CSS
parsing, computed styles, static layout, and software rendering for text and
images. Value binding and progress painting are implemented; CLI integration
is still under development.

See the [execution plan](widget-renderer-plan.md) for implementation phases and
completion criteria.

## Dashboard File

A dashboard is a TOML file with display options and one root widget:

```toml
[dashboard]
width = 960
height = 376
stylesheet = "dashboard.css"
background = "#101318"
fonts = [
  "../../../fonts/DejaVuSans.ttf",
  "../../../fonts/HarmonyOS_Sans_SC_Bold.ttf",
]

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
min = 0
max = 100
```

Paths are relative to the dashboard file. Unknown fields, duplicate IDs,
invalid widget fields, missing files, zero display dimensions, and invalid
progress ranges are rejected during loading.

Font files are explicit dashboard assets. This avoids host font discovery and
makes output reproducible on NixOS and other systems. A missing configured font
or image is reported with its resolved path. If `font-family` does not match a
loaded face, `cosmic-text` applies its normal fallback matching within the
loaded font database.

## Widgets

The configuration model currently accepts:

| Type | Required fields | Optional fields | Children |
|---|---|---|---|
| `row` | none | `id`, `class` | yes |
| `column` | none | `id`, `class` | yes |
| `stack` | none | `id`, `class` | yes |
| `text` | `text` | `id`, `class` | no |
| `image` | `source` | `id`, `class` | no |
| `spacer` | none | `id`, `class` | no |
| `progress` | `value` | `id`, `class`, `min`, `max`, `orientation` | no |

`row` and `column` normalize to a common flex widget with different
directions. Widget IDs must be unique. IDs and classes may contain ASCII
letters, digits, `_`, and `-`.

`progress.orientation` accepts `horizontal` or `vertical`. Its default range is
0 through 100 and its default orientation is horizontal.

Text and progress values support parsed sensor interpolation:

```text
{{ sensor_name }}
{{ sensor_name | default("N/A") }}
{{ sensor_name | number(0) }}
{{ sensor_name | number(1) }}
```

Text may contain multiple interpolations mixed with literal content. Missing
values resolve to an empty string unless `default` is specified. The `number`
filter rounds finite numeric values to 0 through 10 decimal places. Invalid
binding syntax is rejected during dashboard loading.

Pass sensor values using `ValueMap`, which is compatible with the existing
`HashMap<String, String>` sensor state:

```rust
let image = renderer.render_with_values(&dashboard, &values)?;
```

Progress widgets clamp resolved numeric values to `min` and `max`. Missing
values use `min`; malformed present values return an error identifying the
widget path. `background-color` paints the track and `color` paints the fill.
Vertical progress fills from bottom to top.

## One-Shot Preview

`asterctl` can render a deterministic dashboard preview without opening a
serial device:

```shell
asterctl \
  --dashboard examples/dashboards/system-overview/dashboard.toml \
  --sensor-path examples/dashboards/data/system-values.txt \
  --render-once \
  --save
```

This writes exactly one image to `out/dashboard.png` and exits. Continuous
dashboard display mode is not implemented yet.

## Stylesheet Contract

Stylesheets support these selectors:

```css
text {}
.metric {}
text.metric {}
#dashboard {}
```

Selector lists, descendant selectors, child selectors, attributes,
pseudo-classes, and at-rules are rejected.

Supported layout properties:

- `display`, `flex-direction`, `flex-grow`, `flex-shrink`
- `width`, `height`, `min-width`, `min-height`, `max-width`, `max-height`
- `gap`, `margin`, `padding`
- `align-items`, `align-self`, `justify-content`

Supported paint and content properties:

- `color`, `background-color`, `opacity`
- `border-width`, `border-color`, `border-radius`, `overflow`
- `font-family`, `font-size`, `font-weight`, `line-height`
- `text-align`, `text-overflow`, `white-space`
- `object-fit`, `object-position`

Lengths accept `px`, percentages, unitless zero, and `auto` where applicable.
`margin` and `padding` currently accept one value for all four sides.
Unsupported properties and values are errors.

The cascade order is widget type, class, type plus class, then ID. Later rules
win when specificity is equal. Text color, family, size, weight, line height,
and alignment inherit from the parent.

## Static Layout

`Dashboard::compute_layout` produces an owned tree of absolute pixel
coordinates. Flex containers use Taffy. Stack children share the parent content
box and paint in configuration order. Text and images contribute intrinsic
sizes when dimensions are `auto`.

Create one `Renderer` per loaded dashboard and reuse it so decoded images,
loaded fonts, and rasterized glyphs remain cached:

```rust
let dashboard = aster_ui::Dashboard::load("dashboard.toml")?;
let mut renderer = aster_ui::Renderer::new(&dashboard)?;
let image: image::RgbaImage = renderer.render(&dashboard)?;
```

The renderer paints dashboard and widget backgrounds, borders and rounded
corners, text, images, and progress fills. It supports inherited opacity,
rectangular `overflow: hidden` clipping, text alignment and wrapping, and all
four `object-fit` modes. Rounded overflow clipping and
`text-overflow: ellipsis` remain future work.

## Examples

Reference configurations and representative sensor maps are under:

```text
examples/dashboards/
  data/
  storage-overview/
  system-overview/
```
