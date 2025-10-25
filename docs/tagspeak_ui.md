# TagSpeak UI

This document describes the current TagSpeak UI packets, behaviors, and adapter rules as implemented in this repository. It consolidates what we built: inline layout sugar, equal‑split defaults, borders, and scope semantics.

---

## Quick Start

- Build with the egui adapter enabled:
  - `cargo run --features ui_egui -- run examples/basics/ui/window_demo.tgsk`
- Debug overlay (shows region ids and debug borders):
  - `TAGSPEAK_UI_DEBUG=true cargo run --features ui_egui -- run …`

---

## Packets

- `app` — top‑level UI container
  - `[app@"Title"]{ … }`
  - Renders a window; children are frames and elements.

- `frame` — structural region
  - `[frame:id@"Label"]{ … }`
  - An addressable region id; targetable by `[layout(... )@id]`.

- `label`, `button`, `textbox`, `checkbox`, `separator`, `spacer`
  - Straightforward primitives mapped to egui widgets.

- `layout` — target form
  - `[layout(params)@target_id]`
  - Applies layout attributes to a region by id. See “Layout Params”.

- `layout` — inline wrapper sugar
  - `[layout(params)]{ … }`
  - Creates an anonymous region that applies `params` to its enclosed children and passes that region downstream (inside the current parent). Location is ignored in this sugar (it does not “pop” to a parent slot).
  - Optional scope id: `[layout(params)@my_scope]{ … }`.

- `scope` — UI context capture (already present)
  - `[scope@"name"]{ … }` captures stores in the body as context‑bound (predicate on `__ui_scope=="name"`).
  - During rendering, the adapter sets `__ui_scope` to the current region id. Button clicks capture this id and evaluate follow‑up calls with `__scope_capture` set so context‑bound stores write to the right variant.

- `funct` / `call` — define and invoke UI actions
  - `[funct:tag]{ … }` and `[call@tag]` as usual.
  - Actions run after the UI frame; see adapter notes below.

- `var` — resolve a runtime variable (added)
  - `[var@name]` returns the current value of a variable (Num/Str/Bool/Unit). Used by conditionals and comparisons.

---

## Layout Params

Applies to both target and inline sugar forms unless noted.

- `direction = horizontal|vertical`
- `location  = top|bottom|left|right|center` (target form only)
- `order     = <u32>` (sibling sort hint)
- `behavior  = flex | grid(cols,rows)`
- `spacing   = <px>` (item spacing)
- `padding   = <px>` (inner margin)
- `align     = start|center|end`
- `width     = fill | <px>`
- `border    = <px>` (stroke width; 0 disables)
- `border_color = "#RRGGBB[AA]"` (hex; alpha optional)

Notes:
- Inline `[layout]{ … }` ignores `location` to keep the wrapper inside its parent region.
- Target `[layout(... )@id]` applies to the addressed region; `location` is meaningful there.

---

## Adapter Rules (egui)

- Region width defaults
  - Center regions: when explicitly marked `location=center` and no `width` is set, the adapter claims available width for that region.
  - Inline wrappers and other locations do not auto‑fill unless `width=fill` is set.

- Horizontal equal‑split (new default)
  - For regions with `direction=horizontal`, children evenly share available width by default.
  - If any child has `width=<px>`, the container falls back to natural left‑to‑right sizing to honor explicit sizes.

- Grid behavior
  - `behavior=grid(cols,rows)` renders children in an egui `Grid` with `cols` columns. `rows` is parsed and preserved in intents for future use; current rendering uses column count.

- Borders
  - When `border>0` or `TAGSPEAK_UI_DEBUG=1`, the region is wrapped in an egui `Frame` with a stroke.
  - `border_color` uses `#RRGGBB` or `#RRGGBBAA`. Default debug color is a blue tone.
  - Borders work with and without padding/spacing; when spacing‑only is present, the adapter still wraps with a stroke if a border is requested.

- Debug overlay
  - `TAGSPEAK_UI_DEBUG=1|true|y|yes` renders region ids and draws a debug border if none specified.

- Actions and scope
  - During render of a region, `__ui_scope` is set to the region id; restored afterward.
  - Button clicks push a `call` into a small queue along with the captured scope id. On the next frame, the adapter evaluates `[call@fn]` with `__scope_capture` set so `[store]` inside that action can write context‑bound entries.

---

## Conditionals & Vars

- Comparisons in `[if@(…)]` and friends can use numbers, strings, idents, or nested packets.
- We extended the conditional parser so:
  - Unquoted strings like `x` are parsed as `[var@x]` (resolve runtime variable).
  - Quoted strings like `"center"` become a string literal.
  - This enables expressions such as `__ui_scope=="center"` to work in routing and guards.

---

## Examples

Horizontal split with two sub‑frames (default equal split):

```tgsk
[app@"Tag Counter"]{
  [frame:main@""]{
    [layout(location=center, direction=horizontal, spacing=12)]{
      [frame:ones@""]{
        [layout(direction=vertical, align=center, spacing=6, padding=4, border=1, border_color="#4a90e2")] {
          [label@"Ones Counter"]
          [button@"Add one"]{ [call@add_one] }
          [button@"Minus one"]{ [call@minus_one] }
        }
      }
      [frame:fives@""]{
        [layout(direction=vertical, align=center, spacing=6, padding=4, border=1, border_color="#e24a6d")] {
          [label@"Fives Counter"]
          [button@"Add five"]{ [call@add_five] }
          [button@"Minus five"]{ [call@minus_five] }
        }
      }
    }
  }
}
```

Targeted layout applied to a named frame:

```tgsk
[frame:sidebar@"Sidebar"]{ … }
[layout(location=left, width=200, padding=8)@sidebar]
```

Inline wrapper for local layout without changing parent placement:

```tgsk
[layout(direction=horizontal, spacing=8, border=1)]{
  [label@"A"]
  [label@"B"]
}
```

Scope‑bound storage inside a region‑scoped action:

```tgsk
[scope@"main"]{
  [button@"Count"]{ [call@inc] }
}
[funct:inc]{ [math@counter+1]>[store@clicks] }
```

---

## Notes & Safety

- All UI scripts run inside the TagSpeak “red box” (`red.tgsk`).
- External operations (`exec`, `http`) remain gated and are unrelated to UI packets.
- Files and network observe the Box Rule as documented in the project root docs.

---

## Changelog (UI‑related)

- Added inline `[layout]{…}` sugar with the same params as targeted form (location ignored in sugar).
- Added `border` and `border_color` layout parameters.
- Default equal‑split for horizontal regions; auto‑claims width for explicit `location=center` when no width is set.
- Added `[var@name]` packet and conditional parsing for idents/strings in comparisons.

