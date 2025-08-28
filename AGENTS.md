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

5. **Current Scope**
   *
   *


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
