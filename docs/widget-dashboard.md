# Widget Dashboards

Widget dashboards are the planned headless layout mode for `asterctl`. The
`aster-ui` crate currently provides configuration validation, strict CSS
parsing, computed styles, and static layout. Painting, value binding, and CLI
integration are still under development.

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

Binding strings are stored but not evaluated yet.

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
box and paint in configuration order. Leaf widgets without explicit dimensions
have zero intrinsic size until text and image measurement are implemented.

## Examples

Reference configurations and representative sensor maps are under:

```text
examples/dashboards/
  data/
  storage-overview/
  system-overview/
```
