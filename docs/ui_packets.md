# UI Packets

These packets provide simple, human-facing console UI without leaving the TagSpeak box. They are namespaced as `ui:` and live under `src/packets/ui/`.

All UI packets pass a value downstream and respect `TAGSPEAK_NONINTERACTIVE=1` by default-denying interactions.

## GUI mode (egui)

- Enable the GUI by building with feature `ui_egui`:
  - `cargo run --features ui_egui -- examples/basics/ui/alert.tgsk`
  - `cargo run --features ui_egui -- examples/basics/ui/select.tgsk`
- Without the feature, packets use console I/O (current default).

## [ui:alert]

- Purpose: Show a message to the operator, then pass through the prior value unchanged.
- Signatures:
  - `[ui:alert@"message"]`
  - `[ui:alert]` (prints the prior value)
- Example:
  - `[msg@"Hello"]>[ui:alert@"This is an alert"]>[print]`
  - With `ui_egui`, this shows a window with an OK button.

## [ui:select]

- Purpose: Present a numbered menu and return the selected option as a string.
- Signature: `[ui:select@"opt1|opt2|opt3"]`
  - Also accepts `@ident` that resolves to a `"a|b|c"` string, or falls back to the prior value if it’s such a string.
- Returns: `"optN"` or `()` if cancelled/noninteractive.
- Example:
  - `[ui:select@"red|green|blue"]>[store@color]`
  - With `ui_egui`, this shows a window with radio list and OK/Cancel.
