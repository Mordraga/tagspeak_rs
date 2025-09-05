# Contributing New Packets

This guide codifies the Packet Additions rules (AGENTS.md §17) so new packets stay human‑friendly, machine‑precise, and safe.

## Checklist

- Avoid duplicates: verify an equivalent packet doesn’t already exist.
  - Run `[reflect(packets)]>[log(json)@/docs/PACKETS.json]` (see `examples/docs/update_packets_json.tgsk`), or inspect `docs/PACKETS.json`.
- Inputs/outputs: specify what the packet consumes and what it returns downstream.
- Side‑effects: packets with effects (print/log/save/exec/http/run) must pass a value through unless explicitly designed as sinks.
- Safety gating:
  - File I/O: resolve paths via the red.tgsk box; never escape root.
  - Exec: require yellow gating or `.tagspeak.toml` allow/allowlist.
  - HTTP: default‑deny; enable/allow via `.tagspeak.toml [network]`.
- Names: short, intuitive, and non‑overlapping; prefer composition of existing packets over new primitives.
- Docs: add at least one canonical example, and sugar example(s) if applicable.

## Packet Definition Template

- Name: `[name]` (and sugar, if any)
- Purpose: one‑sentence description of behavior and value.
- Input: last‑value type(s) and `@arg` type(s) accepted.
- Output: value produced/passed downstream.
- Safety: file/network/exec gating behavior (if any).
- Examples:

Canonical
```tgsk
[name@arg]
```

Sugar (optional)
```tgsk
[sugarform]
```

## Verifying Against The Box

- Ensure operations resolve against the project root containing `red.tgsk`.
- Use `kernel::fs_guard::resolve` for any filesystem paths.
- For network calls, read `.tagspeak.toml` using `kernel::config` and enforce default‑deny semantics.

## Implementation Notes

- Add modules under `src/packets/` (respecting existing grouping: `core`, `files`, `flow`, `execs`).
- Do not overwrite `router.rs`; only extend handlers.
- Prefer minimal, composable behavior; keep grammar small and move behavior into packet code.

