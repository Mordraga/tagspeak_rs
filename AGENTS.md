# AGENTS.md

## Purpose

This repository defines **TagSpeak (DSL)**: a dataflow-oriented packet language designed for AI ↔ human accessibility.
Codex should treat `.tgsk` files as **DSL**, not general-purpose code.

---

## Instructions for Codex

1. **Parsing**

   * Everything is a `[packet]`.
   * Flow is data-driven, not imperative.
   * Inline sugar (like `==`) coexists with canonical packets (e.g. `[eq]`).

2. **File Types**

   * `.tgsk` → TagSpeak source files.
   * `.json` → runtime/memory store.
   * `.yaml` / `.yml` → runtime/memory store.
   * `.toml` → runtime/memory store.
   * `.rs` → interpreter/compiler backend (Rust).

3. **Behavior**

   * Do not “correct” DSL syntax to Python/Rust.
   * When asked for code examples, prefer `[math@5+5]` style over verbose boilerplate.
   * Normalize to LF endings.
   * For file-level creation/overwrites, prefer `[log(json|yaml|toml)@path]{...}`.
   * Packets should be filtered through mod.rs for modularity purposes instead of imported directly.

4. **Safe Edits**

   * Append new packet modules under `src/packets/`.
   * Never overwrite `router.rs` wholesale — modify only within handlers.

---

## Packet Status

### ✅ Implemented

* `[msg@"string"]` → create a string literal.
* `[int@int]` → create an integer.
* `[bool@true|false]` → create a bool value.
* `[note@"message"]` → inline documentation.
* `[math@expr]` → evaluate math expression.
* `[store@name]` → save last value under variable name.
* `[print]` → print the last value (or `[print@value]`).
* `[save@file]` → save current runtime state to file.
* `[load@file/path/relative/red.tgsk]` → load values/config from a file.
* `[log@file.json]` → dump last value to a JSON file (quick + dirty mode).
* `[mod@var]{...}` → edit an in-memory document. Operators:
  * `comp` → set value if absent.
  * `comp!` → overwrite existing value.
  * `merge` → deep merge object structures.
  * `del` → delete field/path.
  * `ins` → insert a new value.
  * Note for these: They are used within {...} of `[mod]`
* `[loopN]{...}` → repeat enclosed block. (allows for `[loopN@funct_name]`)
* `[if(condition)]{...}` → conditional execution if true.
* `[or(condition)]{...}` → else-if style branching.
* `[else]{...}` → fallback branch for conditionals.
* `[funct@name]{...}` → define a reusable function.
* `[log(json|yaml|toml)@file]{...}` → structured logging mode. 
  * `[key(name)@value]` → insert a key/value pair in a structured `[log]` block.
  * `[sect@section]{...}` → create a nested object/table (JSON/YAML/TOML style).
* `[call@funct_name] → call function directly

### 🛠️ In Progress / Planned

---

### ⚠️ Issues
Possible issue with load file paths.

## Features

### Planned
#### Editor / VS Code niceties

Run selection / run file: execute the highlighted chain and show output in an integrated panel.

Inline probes: hover a > and see the last value that flowed through (ghost text; opt-in).

Outline view: a “Flows” tree that lists chains/blocks; click to jump.

Hover docs: packet docs + examples pulled from your rust docstrings (no codegen, just display).

Formatter: align > pipes, normalize spaces/newlines, enforce trailing-comma rules (one-click “Fix all”).

Quick Fixes: on errors, offer actions like “set project root”, “open allowlist”, “dry-run this block”.

Run tagspeak within VS code.

#### Debuggability & safety (no syntax changes)

Sourcemaps: runtime → source mapping with caret highlight on the exact packet that threw.

Stepper: step-over/into blocks; show a timeline of values (non-mutating preview).

Dry-run mode: simulate, show would-write and would-modify diffs for file ops.

Flow IDs: each execution path gets an ID; logs, probes, and colors all match that ID.

#### Visualization

Graph view: live DAG render of the current file (click a node to jump in the editor).

Data lineage: pick a var and see where it’s written/read across the file (small side panel).

Export: one-click Graphviz/DOT or PNG of the flow for docs.

Linting (friendly, not bossy)

Unreachable branch hints (e.g., mutually exclusive guards).

Unused values: warn when a chain produces a value that never gets consumed.

FS guardrails: warn on writes outside the project allowlist; quick-add to allowlist.

#### Testing & stability

Golden tests: .tgsk.test files (inputs + expected outputs) with snapshot update cmd.

Fuzz the tokenizer: built-in fuzz runner for bracket/quote edge cases; saves crashing seeds.

Minimal repro bundle: command that zips the failing snippet + env into a sharable case.

#### Project ergonomics

Sidecar config: .tgskrc for root, env vars, allowlists, and default modes (dry-run, probes).

Examples launcher: list & run anything in examples/ with one click, capture outputs to /out.

Perf pulse: tiny footer showing per-packet timings + total run (helps spot hot spots).

#### Accessibility & polish

SR-friendly errors: structured messages with code, one-line summary, and full detail on expand.

Copy-ready snippets: every error/output block has a copy button and a “Explain run” expand.

### Implimented

## Do Not Touch

* **`router.rs`** — never overwrite wholesale. Only extend handlers.
* **Core runtime (`Runtime`, `Value`, `Packet`)** — do not redefine base types.
* **AST definitions (`ast.rs`)** — structural integrity must remain stable.

---

## Examples

### Quick Dump

```tgsk
[math@1+1]>[log@out.json]
```

Produces `out.json`: `2`

### JSON Structured Log

```tgsk
[log(json)@profile.json]{
  [key(name)@"Saryn"]
  [key(age)@25]
  [key(active)@true]
}
```

### YAML Structured Log

```tgsk
[log(yaml)@profile.yaml]{
  [key(name)@"Saryn"]
  [key(age)@25]
}
```

### TOML Structured Log with Sections

```tgsk
[log(toml)@Cargo.toml]{
  [sect@package]{
    [key(name)@"tagspeak"]
    [key(version)@"0.1.0"]
  }
  [sect@dependencies]{
    [key(anyhow)@"1"]
    [key(serde)@"1"]
  }
}
```

---

## Design Principles

* **Human-friendly** but machine-precise.
* **Composable** — packets can nest and chain.
* **Extensible** — new packets can be added incrementally.
