# Widget Renderer Execution Plan

## Objective

Build an opinionated, headless widget renderer for `asterctl` that replaces
absolute-positioned AOOSTAR sensor fields with reusable layout widgets.

The renderer will:

- read a declarative widget configuration;
- resolve widget bindings from the existing sensor value map;
- apply a documented subset of CSS;
- compute layout for a fixed-size display;
- render a complete RGBA frame;
- pass that frame to the existing `asterctl-lcd` transport.

The first target is the AOOSTAR display size of 960 x 376 pixels. The renderer
must not require GTK, X11, Wayland, a browser engine, or a graphical session.

## Design Decisions

### Keep Data Collection Separate

Widget definitions will not execute commands or own polling tasks. The existing
sensor providers and file watcher remain responsible for collecting values.

Each data-source update changes the shared value map and invalidates the frame.
The render coordinator debounces invalidations, renders the complete widget
tree, and sends the result to the display. The existing frame cache in
`asterctl-lcd` remains responsible for avoiding transmission of unchanged image
chunks.

### Use a Small Widget Model

The configuration format is not intended to be compatible with HTML, Eww, or a
general-purpose GUI toolkit. Only widgets useful on a non-interactive status
display will be supported.

The initial primitives are:

- `row`: horizontal flex container;
- `column`: vertical flex container;
- `stack`: paints children on top of each other;
- `text`: shaped and styled text;
- `image`: raster image with selectable fit behavior;
- `spacer`: empty flexible or fixed-size element;
- `progress`: horizontal or vertical progress indicator.

Later primitives may include:

- `circular-progress`;
- `graph`;
- `gauge`;
- `conditional`;
- reusable application-defined composite widgets.

Interactive controls, event handlers, scrolling, animations, and arbitrary
scripts inside widgets are out of scope.

### Use Full-Frame Rendering

Every invalidation computes layout and paints a complete 960 x 376 frame.
Subtree-level dirty tracking is deferred until profiling demonstrates a need.
This keeps layout, clipping, alpha blending, and widget state deterministic.

### Treat CSS as a Defined Subset

The stylesheet language will use familiar CSS syntax, but only documented
selectors and properties are valid. Unsupported syntax must produce a
configuration error instead of being silently ignored.

This project does not aim to reproduce browser or GTK CSS behavior.

## Proposed Crate Structure

Create a new workspace crate:

```text
crates/aster-ui/
  Cargo.toml
  src/
    lib.rs
    config.rs
    binding.rs
    value.rs
    widget.rs
    style/
      mod.rs
      parser.rs
      selector.rs
      computed.rs
    layout.rs
    paint/
      mod.rs
      canvas.rs
      text.rs
      image.rs
      progress.rs
    assets.rs
    error.rs
```

Responsibilities:

- `config`: deserialize and validate dashboard configuration;
- `binding`: parse and resolve value interpolation;
- `value`: typed view over the sensor value map;
- `widget`: normalized runtime widget tree;
- `style`: parse rules, match selectors, and produce computed styles;
- `layout`: translate widgets and styles to layout nodes;
- `paint`: rasterize laid-out widgets into an RGBA image;
- `assets`: cache fonts and images;
- `error`: diagnostics containing file, widget, selector, and property context.

`aster-ui` must not depend on serial-port code, file watchers, or CLI types.
Its primary API should be approximately:

```rust
pub struct Dashboard {
    // Parsed widget tree, stylesheet, and asset references.
}

pub struct Renderer {
    // Font, image, layout, and paint caches.
}

impl Dashboard {
    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self>;
}

impl Renderer {
    pub fn render(
        &mut self,
        dashboard: &Dashboard,
        values: &ValueMap,
        size: (u32, u32),
    ) -> anyhow::Result<image::RgbaImage>;
}
```

The exact error types should be defined in `aster-ui`; `anyhow` is shown only
to illustrate the boundary.

## Configuration Format

Use TOML for the first version. It integrates with Serde, is readable without a
custom parser, and supports arrays of nested tables.

Example:

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
type = "column"
class = ["metric"]

[[root.children.children]]
type = "text"
class = ["metric-label"]
text = "CPU"

[[root.children.children]]
type = "text"
class = ["metric-value"]
text = "{{ cpu_percent | number(0) }}%"

[[root.children]]
type = "progress"
class = ["memory-progress"]
value = "{{ memory_usage }}"
min = 0
max = 100
```

The configuration loader must:

1. Resolve the stylesheet and asset paths relative to the dashboard file.
2. Reject unknown widget types and fields.
3. Validate required fields and numeric ranges.
4. Attach a stable internal ID or source path to every widget for diagnostics.
5. Normalize `row` and `column` into a common flex-container representation.

Avoid implementing reusable user-defined widget templates in the first
milestone. Add them only after primitive widgets and layout behavior are
stable. Initial reuse can be achieved by keeping dashboards small or generating
TOML outside the renderer.

## Value Bindings

Bindings read from the existing `HashMap<String, String>` sensor state.

The first binding syntax supports:

```text
{{ sensor_name }}
{{ sensor_name | default("N/A") }}
{{ sensor_name | number(0) }}
{{ sensor_name | number(1) }}
```

Rules:

- Plain strings without `{{ ... }}` are constants.
- Multiple bindings may appear in one text string.
- Missing values resolve to an empty string unless `default` is specified.
- Numeric widget properties report a diagnostic when the resolved value is not
  numeric.
- Binding evaluation must not execute shell commands or access the filesystem.

Implement this as a small parser over interpolation segments, not regular
expression replacement. Preserve parsed bindings in the dashboard so they are
not reparsed each frame.

Conditional expressions and arithmetic are deferred. When added, use a small
typed expression model rather than embedding a scripting language.

## CSS Contract

### Selectors

Version one supports:

```css
text {}
.metric {}
#dashboard {}
text.metric {}
```

Selector specificity, from lowest to highest:

1. widget type;
2. class;
3. type plus class;
4. ID.

For equal specificity, the later declaration wins. Inline style fields, if
added later, override stylesheet rules.

Do not initially support:

- descendant or child selectors;
- selector lists;
- attribute selectors;
- pseudo-classes or pseudo-elements;
- media queries;
- keyframes;
- custom properties.

### Properties

Layout properties:

- `display`: `flex`, `stack`, or `none`;
- `flex-direction`: `row` or `column`;
- `flex-grow`;
- `flex-shrink`;
- `width`, `height`;
- `min-width`, `min-height`;
- `max-width`, `max-height`;
- `gap`;
- `margin`;
- `padding`;
- `align-items`;
- `align-self`;
- `justify-content`.

Paint properties:

- `color`;
- `background-color`;
- `opacity`;
- `border-width`;
- `border-color`;
- `border-radius`;
- `overflow`: `visible` or `hidden`.

Text properties:

- `font-family`;
- `font-size`;
- `font-weight`;
- `line-height`;
- `text-align`;
- `text-overflow`: `clip` or `ellipsis`;
- `white-space`: `normal` or `nowrap`.

Image properties:

- `object-fit`: `fill`, `contain`, `cover`, or `none`;
- `object-position`.

Supported lengths:

- integer or decimal pixel values such as `16px`;
- percentages where the layout engine has a defined containing size;
- `auto`;
- unitless zero.

Shorthand support in version one is limited to one-value `margin` and
`padding`. Four-side shorthands can be added after the computed-style model is
tested.

### Inheritance

Only these properties inherit:

- `color`;
- `font-family`;
- `font-size`;
- `font-weight`;
- `line-height`;
- `text-align`.

All other properties use explicit initial values.

## Rendering Dependencies

Use:

- `taffy` for flex layout and measurement callbacks;
- `cosmic-text` for font discovery, shaping, fallback, text measurement, and
  glyph rasterization;
- `tiny-skia` for backgrounds, rounded rectangles, borders, clipping, and
  progress graphics;
- `image` for image decoding, scaling, and the final `RgbaImage`.

Pin compatible crate versions in `Cargo.lock`. Avoid exposing dependency types
through the public `aster-ui` API.

The renderer must be software-only and deterministic at a fixed font set,
display size, configuration, and value map.

## Layout Pipeline

Each frame follows these steps:

1. Resolve all dynamic bindings into typed widget properties.
2. Match stylesheet selectors and compute inherited styles.
3. Build or update the Taffy layout tree.
4. Measure text and intrinsic image sizes through layout callbacks.
5. Compute layout with the display dimensions as the root constraint.
6. Traverse the laid-out widget tree in paint order.
7. Paint backgrounds, borders, content, and overlays into an RGBA buffer.
8. Return an `RgbaImage` to the caller.

Coordinates must be converted to physical pixels consistently. Use a scale
factor of `1.0`; CSS pixel values map directly to display pixels.

Round layout coordinates at the paint boundary, not while computing layout.
Document and test the selected rounding rule to prevent one-pixel instability
between frames.

## Text Rendering

The existing `imageproc`/`ab_glyph` path uses manual size and vertical-position
adjustments. The new renderer should not carry those adjustments forward.

Text implementation requirements:

- measure and render through the same shaping engine;
- support UTF-8, bidirectional text, font fallback, and common CJK glyphs;
- cache loaded font data and shaped runs where practical;
- clip or ellipsize according to computed style;
- align text within the layout content box;
- render alpha-blended glyphs into the frame.

Tests must use repository-controlled fonts to avoid host-dependent snapshots.

## Runtime Integration

Add a new CLI mode without removing the existing AOOSTAR panel mode during
development. A tentative interface is:

```shell
asterctl --dashboard dashboard.toml
```

Optional development flags:

```shell
asterctl --dashboard dashboard.toml --simulate --save
asterctl --dashboard dashboard.toml --render-once --save
```

Runtime behavior:

1. Load and validate the dashboard before opening the serial device where
   practical.
2. Start the existing sensor file watcher.
3. Render an initial frame from currently available values.
4. Receive invalidation notifications when sensor values change.
5. Debounce notifications for a configurable 20-50 ms interval.
6. Render and send one frame for the combined changes.
7. Watch dashboard, stylesheet, and asset files in development mode and reload
   them atomically.

On reload failure, retain the last valid dashboard and report the diagnostic.
Do not replace the display with a partially parsed frame.

## Delivery Phases

### Phase 0: Baseline and Fixtures

Deliverables:

- capture representative sensor maps;
- add two reference dashboard designs;
- record current render and send timings in release mode;
- establish deterministic repository fonts and test images;
- document the initial CSS and widget contracts.

Completion criteria:

- fixtures can be loaded without the LCD attached;
- expected 960 x 376 output images are checked into test fixtures or generated
  reproducibly;
- baseline timings are recorded in the development notes.

### Phase 1: Crate Skeleton and Static Layout

Deliverables:

- create `aster-ui`;
- deserialize and validate TOML;
- parse the CSS subset;
- implement selector matching and computed styles;
- integrate Taffy;
- implement `row`, `column`, `stack`, and `spacer`;
- render backgrounds and borders.

Completion criteria:

- a static nested layout renders to a 960 x 376 PNG;
- invalid widgets, properties, and selectors produce contextual errors;
- layout unit tests cover sizing, padding, gap, alignment, flex growth, and
  stack paint order.

### Phase 2: Text and Images

Deliverables:

- integrate `cosmic-text`;
- implement `text`;
- implement intrinsic text measurement and alignment;
- implement image loading, caching, and object-fit behavior;
- add clipping and opacity.

Completion criteria:

- Latin and CJK fixture text render correctly;
- text measurement matches painted bounds;
- missing fonts and images produce useful diagnostics or documented fallback;
- snapshot tests cover typography, clipping, and each image fit mode.

### Phase 3: Bindings and Progress Widgets

Deliverables:

- implement parsed interpolation bindings;
- add default and numeric formatting filters;
- implement `progress`;
- bind text and progress values to the existing sensor map.

Completion criteria:

- changing a value map entry changes only the expected pixels in fixture
  comparisons;
- missing and malformed values follow documented behavior;
- progress values are clamped to their configured range;
- bindings are parsed once at dashboard load.

### Phase 4: `asterctl` Integration

Deliverables:

- add `--dashboard`;
- connect existing file-based sensor updates;
- implement invalidation debounce;
- send rendered frames through `AooScreen`;
- make `--simulate --save` useful for dashboard development;
- add `--render-once` for deterministic preview generation.

Completion criteria:

- dashboard mode works without a real LCD under `--simulate`;
- one-shot mode writes exactly one PNG and exits successfully;
- repeated unchanged values do not cause unnecessary frame transmission;
- current `--config` behavior remains functional.

### Phase 5: Developer Reload and Diagnostics

Deliverables:

- watch TOML, CSS, fonts, and images;
- reload the dashboard atomically;
- retain the last valid frame after a reload error;
- report source locations for configuration and stylesheet errors where
  available.

Completion criteria:

- editing CSS updates the simulated or attached display;
- invalid edits do not terminate the running process;
- fixing an invalid file resumes rendering without restart.

### Phase 6: Advanced Display Widgets

Implemented:

- `circular-progress`;
- bounded time-series `graph`;
- `gauge`;
- conditional visibility;
- reusable composite widgets.

Each addition requires a documented configuration contract, invalid-input
behavior, rendering tests, and at least one example dashboard.

## Testing Strategy

### Unit Tests

Cover:

- TOML validation;
- CSS tokenization and property parsing;
- selector specificity and cascade order;
- inheritance and initial values;
- binding parsing and formatting;
- value conversion and clamping;
- layout rounding;
- color and length parsing.

### Renderer Tests

Render small deterministic fixtures and compare:

- exact pixels for geometric primitives;
- perceptual or tolerance-based output for anti-aliased text;
- layout rectangles separately from raster output;
- clipping, alpha blending, borders, and rounded corners.

Store expected outputs by renderer feature, not as one large dashboard snapshot,
so failures remain diagnosable.

### Integration Tests

Cover:

- dashboard load to PNG output;
- sensor-map update to new frame;
- invalidation debounce;
- stylesheet and asset reload;
- simulated LCD transmission;
- unchanged-frame behavior.

### Performance Checks

Measure release builds on the target host:

- dashboard load time;
- style resolution time;
- layout time;
- paint time;
- total render time;
- RGB565 conversion time;
- changed chunks and serial send time.

Initial target: complete layout and paint comfortably within 100 ms for a
typical dashboard. This is intentionally conservative for a one-second sensor
refresh interval and should be revised after measurement.

## Error Handling

Configuration and runtime errors must identify:

- source file;
- widget ID or tree path;
- CSS selector and property where applicable;
- invalid value;
- expected value or supported alternatives.

Fatal startup errors:

- unreadable dashboard;
- invalid widget tree;
- invalid stylesheet;
- root size mismatch when strict sizing is enabled.

Recoverable runtime errors:

- missing sensor value;
- temporary asset reload failure;
- malformed sensor numeric value;
- invalid development reload.

Avoid logging the same recoverable error every frame. Rate-limit or suppress
duplicates until the underlying value changes.

## Compatibility and Migration

The existing AOOSTAR JSON renderer remains available while dashboard mode is
developed. Do not attempt an automatic conversion until the new primitive set
can represent both bundled example panels.

After Phase 4:

1. Recreate `cfg/monitor.json` as example dashboard files.
2. Compare visual output and LCD update behavior.
3. Document feature differences.
4. Decide whether AOOSTAR JSON support remains permanent, moves behind a Cargo
   feature, or is deprecated.

The `asterctl-lcd` crate and its public transport API should remain independent
of this migration.

## Explicit Non-Goals

- Full CSS compatibility.
- HTML or DOM support.
- GTK or Eww widget compatibility.
- JavaScript or arbitrary expression execution.
- Interactive input widgets.
- Per-widget polling processes.
- GPU rendering.
- Subtree-level frame transmission.
- A graphical dashboard editor.

## Definition of Done

The initial widget-renderer project is complete when:

- a dashboard is defined using TOML and the documented CSS subset;
- it contains nested flex layouts, styled text, images, and progress widgets;
- values update through the existing sensor file mechanism;
- updates are debounced into complete frame renders;
- frames render headlessly to deterministic 960 x 376 images;
- the same frames can be sent through `asterctl-lcd`;
- preview and one-shot output work with no LCD attached;
- invalid configuration produces actionable diagnostics;
- existing AOOSTAR panel mode has not regressed;
- architecture, supported syntax, examples, and limitations are documented.
