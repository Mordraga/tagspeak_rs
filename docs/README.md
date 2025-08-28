<h1>
  <img src="/misc/Tagspeak.png" alt="TagSpeak Gecko" width="40"/>
  TagSpeak RS
</h1>

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
## Setup
* Make sure Rust and its dependencies are installed.

* In repo, in terminal
### Windows
```bash
cargo build --bin tagspeak_setup
```
build engine, install

###linux
```shell
cargo build --bin tagspeak_setup_linux
```
follow instructions



## Roadmap
- [x] literals/math/[store:rigid][store:context(conditions)]/note
- [x] funct + loop (inline + tag)
- [x] call tags directly (`[call@step]`)
- [x] conditionals (`[if@(x>2)]{...}[else]{...}`)
- [x] load/write/modify files (`[log], [mod], [save], [load]`
- [] modular imports / red.tgsk boundaries

## Why a gecko?
Technically an anole, but lizards are some of the most adaptive and modular animals on the planet besides insects. They are found on every continent besides antartica.
Also, consider: *Lizard. Lizard. Lizard. Lizard.*
<img src="/misc/Tagspeak.png" alt="TagSpeak Gecko" width="15"/>
