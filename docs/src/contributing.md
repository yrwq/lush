# contributing

- keep runtime behavior inside `require("lush")` modules

## widget architecture notes

- all widgets share `WidgetBase` fields for common behavior (size, classes, alignment, visibility)
- widget-specific fields live in `WidgetProps` variants
- shared post-build behavior is centralized via `finalize_widget(...)`
- for `WidgetKind`, avoid duplicate maps: use `from_lua_kind`, `as_lua_kind`, `css_class`, `allowed_fields`

## data architecture notes

- for common system metrics, prefer rust-native providers
- avoid adding shell-command polling in lua for data sources
- keep lua-side `lush.data` as a wrapper and signal composition layer
- providers should be lazy: start on first `use/watch`, stop on last `unuse`

## before commit

- docs include concrete examples
- no compatibility shim unless requested
