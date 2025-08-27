# TagSpeak 101

## What is TagSpeak?

TagSpeak is a **dataflow-oriented DSL** designed for clarity and accessibility between humans and AI models. Unlike imperative languages, TagSpeak flows data through chained packets, each represented as `[packet@arg]` blocks. The goal is to be both **human-readable** and **machine-parsable**.

---

## Core Concepts

1. **Packets**
   Everything in TagSpeak is a `[packet]`. Packets transform, store, or route data.

   Example:

   ```tgsk
   [math@5+5]>[print]
   ```

   → outputs `10`

2. **Dataflow, not imperative**
   Execution flows left to right, through pipes (`>`), carrying values forward.

   Example:

   ```tgsk
   [msg@"hi"]>[store@greeting]
   ```

   → stores `hi` as `greeting`.

3. **Inline sugar + canonical packets**
   Human-friendly syntax (`==`, `>`, etc.) coexists with explicit packets like `[eq]`, `[gt]`, `[and]`.

---

## Packet Types

### Value Packets

* `[msg@"string"]` → create a string literal.
* `[int@int]` → create an integer.
* `[bool@true|false]` → create a bool true|false statement.
* `[note@"message"]` → inline documentation/annotation.

### Function Packets

* `[print]` → print the last value. can also use `[print@value]` to print specific values.
* `[math@expr]` → evaluate math expression.
* `[store@name]` → save last value under variable name.

### File Packets

* `[save@file]` → save current runtime state to file.
* `[load(json|yaml|toml)@file]` → load values/config from a file.
* `[log@file.json]` → dump last value to a JSON file (quick + dirty mode).
* `[log(json|yaml|toml)@file]{...}` → structured logging mode: build and write formatted docs.
* `[mod@var]{...}` → edit an in-memory document previously loaded and saved into a variable. Supports operators:

  * `comp` → set a value if absent.
  * `comp!` → overwrite existing value.
  * `merge` → deep merge object structures.
  * `del` → delete a field/path.
  * `ins` → insert a new value.

### Control Flow Packets

* `[loopN]{...}` → repeat enclosed block `N` times.
* `[cond(condition)]{...}[else]{...}` → conditional branching, run one block if true, else the other.
* `[funct@name]{...}` → define a reusable function.
* `[call@name]` → call a function defined with `[funct]`.

### Structured Logging Helpers

* `[key(name)@value]` → insert a key/value pair inside a structured `[log]` block.
* `[sect@section]{...}` → create a nested object/table (JSON/YAML/TOML style sections).

---

## Examples

### Quick Dump

```tgsk
[math@2+2]>[log@result.json]
```

Produces `result.json`:

```json
4
```

### JSON Structured Log

```tgsk
[log(json)@profile.json]{
  [key(name)@"Saryn"]
  [key(age)@25]
  [key(active)@true]
}
```

Produces `profile.json`:

```json
{
  "name": "Saryn",
  "age": 25,
  "active": true
}
```

### YAML Structured Log

```tgsk
[log(yaml)@profile.yaml]{
  [key(name)@"Saryn"]
  [key(age)@25]
}
```

Produces `profile.yaml`:

```yaml
name: Saryn
age: 25
```

### TOML Structured Log

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

Produces `Cargo.toml`:

```toml
[package]
name = "tagspeak"
version = "0.1.0"

[dependencies]
anyhow = "1"
serde = "1"
```

---

## Design Principles

* **Human-friendly** but machine-precise.
* **Composable** — packets can nest and chain.
* **Extensible** — future packets may add arrays, merges, or other structures.
