# TagSpeak RS
![TagSpeak Gecko](./misc/Tagspeak.png)
TagSpeak is a symbolic, packet-based language designed to be human-readable and machine-parsable.
This Rust implementation (`tagspeak_rs`) parses and executes `.tgsk` scripts.

---

## Core Ideas
- Everything is a packet → `[op@arg]`
- Packets chain with `>` → `[math@2+2] > [print@result]`
- Blocks use `{ ... }` → group multiple packets
- Strings use quotes → `[print@"hello world"]`
- Comments supported → `#`, `//`, `/* ... */` or TagSpeak’s own `[note@]`

---

## Features Implemented
- math → evaluate expressions with `meval`
- store → assign variables
- print → output values or strings
- note → dev/debug annotation
- funct → define named blocks
- call → call a defined function `[call@name]`
- loop →
  - `[loop@3]{ ... }` → inline loop
  - `[funct:step]{ ... } … [loop3@step]` or `[loop:step@3]` → tag loop (reusable)
- load → load JSON/YAML/TOML (by file extension) within the nearest `red.tgsk` sandbox
- log →
  - quick: `[log@file.json]` dumps last value as JSON
  - structured: `[log(json|yaml|toml)@file]{ [key(name)@value] [sect@section]{...} }`
- red.tgsk → Root file marker/sentinel. Must exist in your project root; all file access is sandboxed to this boundary.

...

### Notes
- Ensure a `red.tgsk` file exists in your project root (can be empty).
- All `[load@...]` paths are resolved relative to the nearest `red.tgsk`—files outside this boundary cannot be accessed.
- Example scripts and data files are in the `examples/` directory.

---

## Run

```bash
cargo run -- examples/smoke.tgsk
```

### Testing

```bash
cargo test
```

---

## Roadmap
- [x] math/store/print/note
- [x] funct + loop (inline + tag)
- [x] call tags directly (`[call@step]`)
- [ ] conditionals (`[if@(x>2)]{...}[else]{...}`)
- [ ] modular imports / red.tgsk boundaries

