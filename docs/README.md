<h1>
  <img src="/misc/Tagspeak.png" alt="TagSpeak Gecko" width="40"/>
  TagSpeak RS
</h1>

> A tiny packet‑based language for human ↔ AI workflows. Built in Rust. Calm by design.

---

## What is TagSpeak?

**TagSpeak** is a symbolic, packet‑based language designed to be **human‑readable** and **machine‑parsable**. This Rust implementation (`tagspeak_rs`) parses and executes `.tgsk` scripts.

**Core ideas:**

* **Everything is a packet** → `[op@arg]`
* **Packets chain** with `>` → `[math@2+2] > [print@result]`
* **Blocks** use `{ ... }` to group work
* **Strings** use quotes → `[print@"hello world"]`
* **Comments** are welcome: `#`, `//`, `/* ... */`, or TagSpeak’s own `[note@...]`

If you like small, modular, readable tools: you’re home. 🦎

---

## Features (at a glance)

### Core

* `math` — evaluate expressions with `meval`
* `store` — assign variables (`rigid`, `fluid`, `context(...)` modes)
* `print` — output values/strings; pass‑through friendly
* `note` — inline dev/debug annotation

### Flow

* `funct` — define named blocks: `[funct:step]{ ... }`
* `call` — call a defined function: `[call@step]`
* `loop` —

  * Inline: `[loop@3]{ ... }`
  * Tag loop (reusable): `[funct:step]{ ... } … [loop3@step]` or `[loop:step@3]`
* Conditionals — `[if@(x>2)]{...}[else]{...}` with chainable `[or@(cond)]`

### Files & Data

* `load` — JSON/YAML/TOML (by extension), sandboxed by the nearest `red.tgsk`
* `log` —

  * Quick dump: `[log@file.json]` (last value as JSON)
  * Structured emit: `[log(json|yaml|toml)@file]{ [key(name)@value] [sect@section]{...} }`
* `parse` — parse strings into docs: `[parse(json|yaml|toml)@string]`
* Builders — arrays/objects with `[array]{...}` and `[obj]{ [key(k)@v] ... }`

### Exec & Network

* `exec` — run shell commands (stdout/code/stderr/json modes)

  * `[exec@"cmd"]`, `[exec(code)@"cmd"]`, `[exec(stderr)@"cmd"]`, `[exec(json)@"cmd"]`
* `run` — execute another `.tgsk` inside the same sandbox
* `http` — opt‑in HTTP client packets (`get/post/put/delete`) when network is enabled

### Sandbox & Project Boundary

* **`red.tgsk`** — root marker/sentinel. Must exist in your project root; all file access is sandboxed to the nearest `red.tgsk`.

More details live in **[TagSpeak\_101.md](Tagspeak_101.md)**.

---

## Quick Notes

* Put an (even empty) `red.tgsk` in your project root.
* All `[load@...]` paths resolve inside that red box; outside access is denied.
* Peek `examples/` for runnable scripts.

---

## Run

```bash
cargo run -- examples/smoke.tgsk
```

### Test

```bash
cargo test
```

---

## Setup

Make sure Rust (stable) is installed.

### Windows

```bash
cargo build --bin tagspeak_setup
```

Then follow the installer’s guidance to build the engine and register the CLI.

### Linux

```bash
cargo build --bin tagspeak_setup_linux
```

Follow the printed instructions to complete setup.

---

## Roadmap

* [x] literals / math / `store:rigid` / `store:context(conditions)` / `note`
* [x] `funct` + `loop` (inline + tag)
* [x] call tags directly (`[call@step]`)
* [x] conditionals (`[if@(x>2)]{...}[else]{...}`)
* [x] load/write/modify files (`[log]`, `[mod]`, `[save]`, `[load]`)
* [x] modular imports / `red.tgsk` boundaries
* [x] `[http]` calls
* [x] `[exec]`, `[run]`, `[yellow]` packets in place

---

## Safety

* **Root required** — scripts only run inside a tree with a `red.tgsk` (nearest ancestor). If missing, the runtime aborts with guidance.
* **Yellow prompts** — use `[yellow@"message"]{ ... }` to ask before executing a block.
* **Exec gating** — set `TAGSPEAK_ALLOW_EXEC=1` to auto‑allow `[exec]` (or answer interactively).
* **Run depth** — `[run]` defaults to a max depth of 8 (`TAGSPEAK_MAX_RUN_DEPTH` to override).
* **Non‑interactive** — set `TAGSPEAK_NONINTERACTIVE=1` to disable prompts (operations default‑deny unless allowed).

### Optional: `.tagspeak.toml`

**Location:** project root (next to `red.tgsk`)
**Precedence:** CLI flags > env vars > `.tagspeak.toml` > defaults

**Keys:**

* `security.allow_exec` (bool) — allow `[exec]` without yellow
* `security.exec_allowlist` (array) — commands allowed without yellow (e.g., `["echo","git"]`)
* `run.max_depth` (int) — max nested `[run]` depth
* `run.require_yellow` (bool) — also require yellow for `[run]`
* `prompts.noninteractive` (bool)
* `network.enabled` (bool) — enable outbound HTTP
* `network.allow` (array) — allowlist of prefixes/hosts (e.g., `"https://api.example.com"`, `"*.githubusercontent.com"`)

**Example:**

```toml
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

---

## Packet Reference (Canonical)

### Core / Data

* `[msg@"string"]` — string literal
* `[int@42]` — numeric literal
* `[bool@true|false]` — boolean literal
* `[note@"message"]` — inline annotation (returns Unit)
* `[math@expr]` — evaluate math expression
* `[print]` — print last (or `[print@value]`), pass‑through
* `[store@name]` — save last under `name`. Modes: `[store:rigid@name]`, `[store:fluid@name]`, `[store:context(cond)@name]`
* `[parse(json|yaml|toml)@string]` — parse string into an in‑memory document
* `[array]{ ... }` — build an array from enclosed packets; sugar: `[array@[1,2,3]]`
* `[obj]{ [key(k)@v] ... }` — build an object from `[key]` and `[sect]`
* `[len]` — length of last value; also `[len@var|"text"]`
* `[env@NAME]` — read env var (or Unit if missing)
* `[cd@/path]` — change runtime cwd within red box; returns new cwd
* `[dump]` — pretty‑print last value (docs as pretty JSON); pass‑through
* `[reflect(packets)]` — list canonical packets; `[reflect(packets_full)]` writes `docs/PACKETS.json`

### Files

* `[load@/path/file.(json|yaml|yml|toml)]` — load file into an editable document
* `[mod@handle]{ comp(path)@v | comp!(path)@v | merge(path)@{...} | del(path) | ins(path)@v | push(path)@v }` — edit document
* `[get(path)@handle]` — extract value at `path` from document
* `[exists(path)@handle]` — test whether `path` exists (bool)
* `[save@handle]` — persist document back to original file
* `[log@/path/file.json]` — dump last value as JSON
* `[log(json|yaml|toml)@/path/file]{ [key(name)@v] [sect@section]{...} }` — structured file emit

### Flow

* `[funct:tag]{...}` — define a reusable block
* `[call@tag]` — invoke a function
* `[loopN]{...}` — repeat N times; sugar: `[loop3@tag]`, `[loop:tag@3]`
* `[if@(cond)] > [then]{...} > [or@(cond)] > [then]{...} > [else] > [then]{...}` — conditional dataflow
* `[or@(cond)]` — additional condition/branch in an if‑chain
* `[else]` — final fallback branch
* `[iter@handle]{...}` — iterate arrays; sets `it` and `idx` during body
* Comparators: `[eq@rhs]`, `[ne@rhs]`, `[lt@rhs]`, `[le@rhs]`, `[gt@rhs]`, `[ge@rhs]` — return bool (sugar: `== != < <= > >=`)

### Exec / Network

* `[exec@"cmd"]` — run shell command (stdout string)

  * Modes: `[exec(code)]`, `[exec(stderr)]`, `[exec(json)]`
* `[run@/path/script.tgsk]` — execute another script inside the same red box; depth limited (`TAGSPEAK_MAX_RUN_DEPTH`)
* `[http(get|post|put|delete)@url]{ [key(header.Name)@v] [key(json)@{...}] [key(body)@"..."] }` — HTTP client (requires `.tagspeak.toml` network enabled + allowlist)
* `[confirm@"message"]{...}` — prompt before running a block. Alias: `[yellow@...]`

**Notes:**

* **Box Rule** — all paths are sandboxed to the nearest `red.tgsk`. Missing root ⇒ `E_BOX_REQUIRED`. Escapes ⇒ `E_BOX_VIOLATION`.
* **Execs & network** are opt‑in; use yellow prompts or `.tagspeak.toml` to allow.

---

## About the Dev

Hello! I’m **Saryn** (she/they), the **sole developer and systems designer** behind **TagSpeak**—a tiny packet‑based DSL for human ↔ AI workflows.

### Design Philosophy

* **Consent‑aware tools**
* **Easy to read**
* **Easy to ship**

TagSpeak reflects that:

* **Everything is a packet**
* **Sugar + canonical coexist**
* **Human‑readable, machine‑parsable outputs**

### Why TagSpeak?

* I’m autistic; flow‑based thinking fits me better than control‑centric stacks.
* Systems + dataflow are how my brain works, so I built a language that meets me there.
* Written in **Rust**, with modularity and user‑centricity from day one—the language flows around the user, not the other way around.

I’m happy with where TagSpeak is today, and I’m always open to ideas. If you spot a rough edge or want to contribute, **open an issue or PR**. Thanks for being here. ♥

### Why a Gecko?

- Tiny correction: it’s actually an **anole** (a lizard cousin of geckos)—we just like the gecko vibe.
- Reptiles are famously **adaptable**. They thrive on every continent **except Antarctica** and are scarce in true tundra.
- That’s the TagSpeak energy: small, calm, adaptable.
- Also: 🦎🦎🦎🦎

---

### TL;DR (for skimmers)

* **What:** TagSpeak = packet‑based DSL for human ↔ AI workflows
* **Values:** consent‑aware · readable · shippable
* **Status:** active, welcoming feedback & PRs

---

## Contributing

TagSpeak is open source and welcoming to contributors. Whether you’re here to fix a typo, add examples, or shape core packets, thank you. PRs and issues are open to everyone.

### ND‑Friendly Project Commitments

* Plain‑language docs and examples first
* Predictable formatting and small, reviewable PRs
* Clear issue templates + labels ("good first issue", "needs reproduction")
* No pressure for real‑time replies; async is welcome
* Sensory‑friendly communication: headings, bullets, and code blocks over walls of text

### Ways to Help

* **Docs & examples:** clarify README sections, add runnable `.tgsk` snippets
* **Bugs:** file an issue with steps to reproduce and minimal input
* **Features:** propose changes via an issue first; small, focused PRs are ideal
* **Tests:** add/expand unit tests and integration examples
* **Accessibility:** wording, structure, and UX of messages/errors

### Ground Rules (project values)

* **Everything is a packet.** Prefer composable, minimal packets
* **Sugar + canonical coexist.** If you add sugar, document the canonical form too
* **Readable for humans, parsable for machines.** Favor clarity over cleverness
* **Safety.** No harmful payloads, malware, or unsafe‑by‑default behaviors

### Pull Request Checklist

* [ ] Linked issue (or clear rationale)
* [ ] Docs updated (README or packet docs) with examples
* [ ] Tests added/updated; `cargo test` passes
* [ ] `cargo fmt --all` and `cargo clippy -- -D warnings` pass
* [ ] Changes are scoped and focused (avoid "kitchen sink" PRs)

### Development Setup (Rust CLI)

```bash
git clone <repo-url>
cd tagspeak_rs            # or the repo root
cargo fmt --all
cargo clippy -- -D warnings
cargo test
cargo build --release
```

### VS Code Extension (optional)

```bash
cd vscode-extension
npm i
npm run build             # or use VS Code “Run Extension”
# package: npx vsce package
```

### Issues & Labels

* **good first issue** – safe for newcomers
* **needs reproduction** – missing a minimal repro
* **help wanted** – maintainers welcome outside help
* **docs** / **examples** / **perf** / **bug** – topic tags

### Code of Conduct

We follow the Contributor Covenant. See `CODE_OF_CONDUCT.md`.

### Security

Please report vulnerabilities via GitHub Security Advisories (private) rather than opening a public issue.

---

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
