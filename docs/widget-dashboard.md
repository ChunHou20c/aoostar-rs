# Widget Dashboards

Widget dashboards are the planned headless layout mode for `asterctl`. The
configuration and validation layer is available in the `aster-ui` crate;
layout, CSS parsing, painting, and CLI integration are still under development.

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

The examples include the intended CSS subset, but stylesheet parsing is not
implemented yet. The initial contract is defined in the
[execution plan](widget-renderer-plan.md#css-contract).

Unsupported CSS will be treated as an error once the parser is implemented.

## Examples

Reference configurations and representative sensor maps are under:

```text
examples/dashboards/
  data/
  storage-overview/
  system-overview/
```
