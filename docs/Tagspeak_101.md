# TagSpeak 101

### Audience

This file is a quick reference for **Codex-style agents** and humans who need to read or write TagSpeak (`.tgsk`) programs.

---

## What is TagSpeak?

TagSpeak is a **dataflow-oriented DSL**. Instead of mutating state with imperative statements, values move left → right through chained **packets**. Each packet has the shape `[packet@arg]` and transforms the value it receives.

Data is always carried forward by the `>` connector:

```tgsk
[msg@"hi"]>[store@greeting]>[print]
```

1. `[msg]` produces "hi".

2. `[store]` saves it under `greeting`.

3. `[print]` outputs it.

---

## Core Concepts

1. **Everything is a packet** – packets transform, store, or route data.

2. **Inline sugar vs. canonical packets** – friendly forms (`>` or `==`) have explicit equivalents (`[pipe]`, `[eq]`). Prefer canonical packets when clarity matters.

3. **Structured file operations** – when emitting files, prefer `[log(json|yaml|toml)@path]{...}` over ad‑hoc writes.

---

## Packet Reference (Canonical)

### Core/Data Packets

- `[parse(json|yaml|toml)@string]` — parse a string into an in-memory document (usable by `[mod]`, `[dump]`, `[iter]`).

- `[array]{ ... }` — produce an in-memory JSON array from enclosed packets. Sugar: `[array@[1,2,3]]`.

- `[obj]{ [key(k)@v] ... }` — produce an in-memory JSON object built from `[key]` and `[sect]`.

- `[len]` — length of last value (string length, array length, or object key count). Also `[len@var|"text"]`.
- `[rand]` - random number generator; `[rand]` yields a float in (0,1), while `[rand(min,max)]` evaluates each bound (numbers, vars, or packets) and returns ints when both bounds are whole.

- `[env@NAME]` — read an environment variable; returns Unit if missing.

- `[cd@/path]` — change runtime working directory within the red box; returns new cwd like `/sub/dir`.

- `[dump]` — pretty-print the last value (documents as pretty JSON); pass-through.

- `[mod@doc]{...}` — mutate a loaded document. Sugar packets: `[set(path)@value]`, `[set(path, missing)@value]`, `[remove(path)]`, `[append(list)@value]`, `[merge(meta)@{...}]`. Flags: `[mod(overwrite)@doc]` promotes `comp()` to `comp!()`, and `[mod(debug)@doc]` prints before/after snapshots.

- `[help@packet]` — returns a quick reference string for the named packet. `[help@*]` lists all topics.

- `[lint@/path/script.tgsk]` — runs heuristics that flag lingering notes, unsafe exec usage, and TODO markers. Accepts inline script text as well.

- `[reflect(packets)]` — introspect canonical packets; `[reflect(packets_full)]` writes `docs/PACKETS.json`. Also `[reflect(vars)]`, `[reflect(runtime)]`, `[reflect(doc)@handle]`.

- `[input@"Prompt "]` — read a single line from stdin. Returns the entered string. Respects `TAGSPEAK_NONINTERACTIVE=1` (returns Unit). Sugar: `[input:line@"Prompt "]`.

### Comparators

- `[eq@rhs]` — equality comparator against last value (returns bool). Sugar: `==`.

- `[ne@rhs]` — not-equal comparator. Sugar: `!=`.

- `[lt@rhs]` — less-than comparator. Sugar: `<`.

- `[le@rhs]` — less-than-or-equal comparator. Sugar: `<=`.

- `[gt@rhs]` — greater-than comparator. Sugar: `>`.

- `[ge@rhs]` — greater-than-or-equal comparator. Sugar: `>=`.

### Additional File Packets

- `[get(path)@handle]` — read a value at `path` from a document variable; returns that value (or Unit if missing).

- `[exists(path)@handle]` — returns a bool indicating whether `path` exists in the document.

Notes:
- All file paths resolve inside the nearest `red.tgsk` root; attempts to escape error with `E_BOX_VIOLATION`.

- `[save]`, `[load]`, `[log]`, `[cd]` require a `red.tgsk` present or error with `E_BOX_REQUIRED`.

### Control Flow (Expanded)

- `[loopN]{...}` — repeat enclosed block `N` times. Sugar: `[loop3@tag]`, `[loop:tag@3]`.

- `[if@(cond)] > [then]{...} > [or@(cond)] > [then]{...} > [else] > [then]{...}` — dataflow conditionals with explicit `then` blocks. Comparators and boolean ops allowed in `cond`.

- `[or@(cond)]` — chain additional condition/branch pairs inside an if-chain.

- `[else]` — final fallback branch in an if-chain.

- `[funct:tag]{...}` — define a reusable block under `tag`.

- `[call@tag]` — invoke a function defined with `[funct]`.

- `[iter@handle]{...}` — iterate arrays in a document `handle`; sets `it` (current item) and `idx` (index) during the body.


### Exec Packets

- `[exec@"cmd"]` — run a shell command; returns stdout string. Modes: `[exec(code)@"cmd"]` (exit code), `[exec(stderr)@"cmd"]` (stderr), `[exec(json)@"cmd"]` (JSON string `{code,stdout,stderr}`).
  - Requires a yellow consent block.

- `[run@/path/script.tgsk]` – execute another TagSpeak file inside the same red box; updates cwd relative to that file. Depth limited (default 8, `TAGSPEAK_MAX_RUN_DEPTH`).
- `[tagspeak:run@/path/script.tgsk]` – CLI-flavored wrapper around `[run]`. Sugar: `[tagspeak run@/path/script.tgsk]`. Honors the same yellow + depth guards as `[run]`.
- `[tagspeak:build@/path/script.tgsk]` – parse-check a script without executing; returns `/relative/path` when the syntax is valid.
  Paths starting with `/` are anchored to the current red root (no leading project directory required).

- `[http(get|post|put|delete)@url]{ [key(header.Name)@val] [key(json)@{...}] [key(body)@"..."] }` — outbound HTTP; disabled by default. Enable with `.tagspeak.toml` `[network]` and allowlist hosts.

- `[confirm@"message"]{...}` — prompt before running enclosed block. Env opt-in: `TAGSPEAK_ALLOW_YELLOW=1` to approve all.

- `[yellow@"message"]{...}` — alias of `[confirm]`. Sugar: `[yellow:exec@"cmd"]`, `[yellow:run@"/file.tgsk"]` to gate specific ops.

- `[red@"message"]` — session consent toggle (script-level). Presence of `[red]` in a script enables red for that run.
  - Red gates recursive/meta actions like `[repl]` (red-only).
  - Red does not bypass yellow; use `[yellow]` for per-action consent on dangerous ops.

- `[repl(model) ]{ ... }` — interactive loop (red-only). Prompts `model>` until `exit/quit`.
  - Requires red enabled (`[red@"..."]` first), and does not allow nesting (one REPL per session at a time).
  - Sets `q` to the current input then evaluates the body; prints the body’s output each turn.
  - Example:
    ```tgsk
    [repl(assistant)]{
      # echo back length of input
      [len@q]>[print]
    }
    ```
---

## Examples

### Quick Dump

`tgsk
[math@2+2]>[log@result.json]
`

Produces `result.json`:

```result.json
{
  4
}
```

### JSON Structured Log

```tgsk
[log(json)@profile.json]{
  [key(name)@"Saryn"]
  [key(age)@26]
  [key(tagspeak_dev)@true]
}
```

Produces `profile.json`:

```json
{
  "name": "Saryn",
  "age": 26,
  "tagspeak_dev": true
}
```

### YAML Structured Log

```tgsk
[log(yaml)@profile.yaml]{
  [key(name)@"Saryn"]
  [key(age)@26]
}
```

Produces `profile.yaml`:

```yaml
name: Saryn
age: 26
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

## CLI Commands

- `tagspeak run <file.tgsk>` — execute a script from the shell (same as double-clicking or calling the binary directly).
- `tagspeak build <file.tgsk>` — syntax-check a script without running it; prints `build_ok /relative/path` on success.
- `tagspeak help [packet]` — print inline documentation for a packet (or list the available topics when omitted).
- `tagspeak lint <file.tgsk>` — run the `[lint]` heuristics against a script inside the current red box.

### CLI Sugar Wrapper

```tgsk
[tagspeak run@/basics/data/literals.tgsk]
```

Runs another script using CLI-style sugar. For a syntax check without execution:

```tgsk
[tagspeak build@/basics/data/literals.tgsk]
```

Returns `/basics/data/literals.tgsk` on success.

---

## Design Principles

* **Human-friendly** but machine-precise.
* **Composable** — packets can nest and chain.
* **Extensible** — future packets may add arrays, merges, or other structures.
