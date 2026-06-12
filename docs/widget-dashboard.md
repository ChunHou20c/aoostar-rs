# Widget Dashboards

Widget dashboards are the headless layout mode for `asterctl`. The `aster-ui`
crate provides configuration validation, strict CSS parsing, computed styles,
dynamic layout, sensor bindings, and software rendering for text, images, and
graphical indicators. One-shot, continuous, and live-reload CLI flows are
available.

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
| `circular-progress` | `value` | `id`, `class`, `min`, `max`, `start-angle`, `sweep-angle`, `thickness` | no |
| `graph` | `value` | `id`, `class`, `min`, `max`, `line-width`, `fill` | no |
| `gauge` | `value` | `id`, `class`, `min`, `max`, `start-angle`, `sweep-angle`, `thickness`, `needle-width` | no |
| `conditional` | `value` | `id`, `class`, `equals`, `not-equals` | yes |
| `component` | `component` | `id`, `class` | no |

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

### Circular Progress And Gauges

`circular-progress` draws an arc using `color`. `border-color` draws its track.
The default range is 0 through 100, `start-angle` is -90 degrees,
`sweep-angle` is 360 degrees, and `thickness` is 8 pixels.

`gauge` uses the same range and arc fields, but defaults to a -135 degree
start and a 270 degree sweep. It also draws a needle using `color`;
`needle-width` defaults to 3 pixels.

```toml
[[root.children]]
type = "circular-progress"
value = "{{ cpu_percent }}"
thickness = 6

[[root.children]]
type = "gauge"
value = "{{ cpu_temperature }}"
min = 20
max = 100
needle-width = 2
```

Angles are measured clockwise from the right-hand side of the widget. Negative
sweeps draw counter-clockwise. A sweep must be non-zero and no larger than 360
degrees in either direction.

### Graphs

`graph.value` resolves to samples separated by commas, semicolons, or
whitespace. `color` controls the line, `line-width` defaults to 2 pixels, and
`fill = true` adds a translucent area below the line. If `min` or `max` is
omitted, that bound is derived from the samples.

```toml
[[root.children]]
type = "graph"
value = "{{ cpu_history }}"
min = 0
max = 100
line-width = 2
fill = true
```

### Conditional Visibility

A `conditional` participates in layout only when its condition is true. With
no comparison field, empty values and `0`, `false`, `no`, or `off`
case-insensitively are false. Use either `equals` or `not-equals` for an exact
string comparison.

```toml
[[root.children]]
type = "conditional"
value = "{{ disk_state }}"
not-equals = "healthy"

[[root.children.children]]
type = "text"
text = "Disk requires attention"
```

Hidden branches are not measured or painted.

### Reusable Components

Named components are widget subtrees declared in the top-level `components`
table. A `component` widget expands the template at dashboard load. Instance
classes are appended to the template root classes, and an instance ID is
applied to the expanded root.

```toml
[components.metric-card]
type = "column"
class = ["metric-card"]

[[components.metric-card.children]]
type = "text"
text = "AOOSTAR"

[root]
type = "row"

[[root.children]]
type = "component"
component = "metric-card"
id = "left-card"
class = ["highlighted"]
```

Templates cannot define IDs because using a template more than once would
duplicate them. Component references may be nested, but cycles are rejected.
Templates currently use the same global sensor bindings as the rest of the
dashboard; per-instance component parameters are not supported.

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

This writes exactly one image to `out/dashboard.png` and exits.

Continuous mode watches the sensor input and sends changed frames to the
display:

```shell
asterctl \
  --dashboard examples/dashboards/system-overview/dashboard.toml \
  --sensor-path examples/dashboards/data \
  --simulate \
  --save
```

Sensor changes are debounced for 30 ms. Identical rendered frames are not saved
or transmitted. Saved continuous frames use names such as
`out/dashboard-0001.png`.

Continuous mode also watches the dashboard TOML, stylesheet, configured fonts,
and image assets. A valid edit is loaded atomically and rendered immediately.
If an edit is invalid, `asterctl` logs the error once and keeps the last valid
frame. Fixing the file resumes reload without restarting the process.

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
