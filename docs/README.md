<h1>
  <img src="/misc/Tagspeak.png" alt="TagSpeak Gecko" width="40"/>
  TagSpeak RS
</h1>


#### Developed by Mordraga (Saryn Harris)
---

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
- exec ·+
  - `[exec@"cmd"]` returns stdout as string
  - `[exec(code)@"cmd"]` returns exit code as number
  - `[exec(stderr)@"cmd"]` returns stderr as string
  - `[exec(json)@"cmd"]` returns a JSON string with `{code,stdout,stderr}`
- red.tgsk → Root file marker/sentinel. Must exist in your project root;
all file access is sandboxed to this boundary.
- Among other features. <a href="Tagspeak_101.md">Refer to Tagspeak_101.md for more info </a>

...

### Notes
- Ensure a `red.tgsk` file exists in your project root (can be empty).
- All `[load@...]` paths are resolved relative to the nearest `red.tgsk`—files outside this boundary cannot be accessed.
- Example scripts and data files are in the `examples/` directory.

---

## Run

`
cargo run -- examples/smoke.tgsk
`

### Testing

`
cargo test
`

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
- [x] modular imports / red.tgsk boundaries
- [x] `[http]` calls.
- [x] `[exec], [run], [yellow]` packets enstated

## Safety

- Root required: scripts only run inside a project tree that contains a `red.tgsk` file (nearest ancestor). If absent, the runtime aborts with guidance.
- Yellow prompts: use `[yellow@"message"]{ ... }` to ask before executing a block.
- Exec gating: set `TAGSPEAK_ALLOW_EXEC=1` to auto-allow `[exec]` (or answer interactively).
- Run depth: `[run]` defaults to a max depth of 8; override with `TAGSPEAK_MAX_RUN_DEPTH`.
- Non-interactive: set `TAGSPEAK_NONINTERACTIVE=1` to disable prompts (operations default-deny unless allowed).

### .tagspeak.toml (optional)
- Location: project root (next to `red.tgsk`).
- Precedence: CLI flags > env vars > `.tagspeak.toml` > defaults.
- Keys:
  - `security.allow_exec` (bool) — allow `[exec]` without yellow.
  - `security.exec_allowlist` (array) — commands allowed without yellow (e.g., `["echo","git"]`).
  - `run.max_depth` (int) — max nested `[run]` depth.
  - `run.require_yellow` (bool) — require yellow for `[run]` too.
  - `prompts.noninteractive` (bool).
  - `network.enabled` (bool) — enable outbound HTTP.
  - `network.allow` (array) — allowlist of prefixes or hosts (e.g., ["https://api.example.com", "*.githubusercontent.com"]).

Example:
```
[security]
allow_exec = false
exec_allowlist = ["echo", "git"]

[run]
max_depth = 8
require_yellow = false

[prompts]
noninteractive = false

[network]
enabled = false
allow = ["https://api.example.com", "*.example.org"]
```

## Packet Reference (Canonical)

### Core/Data

- `[msg@"string"]` — string literal.
- `[int@int]` — numeric literal.
- `[bool@true|false]` — boolean literal.
- `[note@"message"]` — inline annotation (returns Unit).
- `[math@expr]` — evaluate math expression.
- `[print]` — print last (or `[print@value]`), pass-through.
- `[store@name]` — save last under `name`. Modes: `[store:rigid@name]`, `[store:fluid@name]`, `[store:context(cond)@name]`.
- `[parse(json|yaml|toml)@string]` — parse string into an in-memory document.
- `[array]{ ... }` — build a JSON array from enclosed packets. Sugar: `[array@[1,2,3]]`.
- `[obj]{ [key(k)@v] ... }` — build a JSON object from `[key]` and `[sect]`.
- `[len]` — length of last value; also `[len@var|"text"]`.
- `[env@NAME]` — read environment variable value (or Unit if missing).
- `[cd@/path]` — change runtime cwd within red box; returns new cwd.
- `[dump]` — pretty-print last value (docs as pretty JSON), pass-through.
- `[reflect(packets)]` — list canonical packets; `[reflect(packets_full)]` writes `docs/PACKETS.json`.

### Files

- `[load@/path/file.(json|yaml|yml|toml)]` — load file into an editable document.
- `[mod@handle]{ comp(path)@v | comp!(path)@v | merge(path)@{...} | del(path) | ins(path)@v | push(path)@v }` — edit document.
- `[get(path)@handle]` — extract value at `path` from document.
- `[exists(path)@handle]` — test whether `path` exists (bool).
- `[save@handle]` — persist document back to original file.
- `[log@/path/file.json]` — dump last value as JSON.
- `[log(json|yaml|toml)@/path/file]{ [key(name)@v] [sect@section]{...} }` — structured file emit.

### Flow

- `[funct:tag]{...}` — define a reusable block.
- `[call@tag]` — invoke a function.
- `[loopN]{...}` — repeat N times. Sugar: `[loop3@tag]`, `[loop:tag@3]`.
- `[if@(cond)] > [then]{...} > [or@(cond)] > [then]{...} > [else] > [then]{...}` — conditional dataflow.
- `[or@(cond)]` — additional condition/branch in an if-chain.
- `[else]` — final fallback branch.
- `[iter@handle]{...}` — iterate arrays; sets `it` and `idx` during body.
- Comparators: `[eq@rhs]`, `[ne@rhs]`, `[lt@rhs]`, `[le@rhs]`, `[gt@rhs]`, `[ge@rhs]` — return bool; sugar `== != < <= > >=`.

### Exec/Network

- `[exec@"cmd"]` — run shell command; stdout string. Modes: `[exec(code)]`, `[exec(stderr)]`, `[exec(json)]`.
- `[run@/path/script.tgsk]` — execute another script inside the same red box; depth limited (`TAGSPEAK_MAX_RUN_DEPTH`).
- `[http(get|post|put|delete)@url]{ [key(header.Name)@v] [key(json)@{...}] [key(body)@"..."] }` — HTTP client (requires `.tagspeak.toml` network enabled + allowlist).
- `[confirm@"message"]{...}` — prompt before running a block. Alias: `[yellow@...]`.

Notes:
- Box Rule: all paths are sandboxed to the nearest `red.tgsk`. Missing root ⇒ `E_BOX_REQUIRED`. Escapes ⇒ `E_BOX_VIOLATION`.
- Execs and network are opt-in; use yellow prompts or `.tagspeak.toml` to allow.

## MIT License

Copyright (c) 2025 Saryn Harris

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
