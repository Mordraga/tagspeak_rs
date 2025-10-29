# TagSpeak Breakers — Playground Report

This report captures observed behavior while intentionally stress‑testing the example breakers under the examples red box.

## Setup

- Built CLI: `cargo build --release` → `target/release/tagspeak_rs.exe`
- Red box: ran from `examples` (uses `examples/red.tgsk` and `examples/.tagspeak.toml`)
- Default runs: no consent flags; network allowlist as in repo
- Consent runs: `TAGSPEAK_ALLOW_YELLOW=1`, `TAGSPEAK_ALLOW_EXEC=1`, `TAGSPEAK_NONINTERACTIVE=1`
- Network tweaks: edited `examples/.tagspeak.toml` allowlist during tests

## Baseline Results (no consent)

- 01_recursive_death_spiral.tgsk → E_CALL_DEPTH_EXCEEDED (256)
- 02_box_escape_artist.tgsk → OS error (path not found) — box intact
- 03_yellow_paradox_direct.tgsk → prompts for confirm (blocked)
- 03_yellow_paradox_indirect.tgsk → prompts for confirm (blocked)
- 04_repl_ourobouros.tgsk → unknown packet `repl(model)` (not implemented)
- 05_storage_collision.tgsk → prints `var_exists`
- 06_parsing_bomb_json.tgsk → JSON parse EOF error
- 06_parsing_bomb_yaml.tgsk → YAML recursion limit exceeded
- 07_http_allowlist_bypass_at.tgsk → E_BOX_VIOLATION (not allowlisted)
- 07_http_path_traversal.tgsk → E_BOX_VIOLATION (not allowlisted)
- 08_empty_everything.tgsk → empty packet diagnostics
- 09_integer_overflow_loop.tgsk → E_LOOP_OVERFLOW (max 1,000,000)
- 10_type_confusion.tgsk → prints checks; later `unknown variable str`
- 11_exec_injection.tgsk → prompts for confirm (blocked)
- 12_file_handle_exhaustion.tgsk → `unsupported_ext:tgsk`
- 13_conditional_confusion.tgsk → malformed packet diagnostics
- 15_scope_leak.tgsk → prints Unit `()`
- 99_nuclear_option.tgsk → unknown packet `repl(model)`

## With Consent Enabled

- 03_yellow_paradox_* → proceed without prompts; no further errors printed
- 11_exec_injection.tgsk → executes without prompt; no error output
- All other guards unchanged (recursion limit, overflow, parse limits, etc.)

## HTTP Breakers — Allowlist Experiments

Config file: `examples/.tagspeak.toml`

Initial allowlist:

```
[network]
enabled = true
allow = [
  "https://httpbin.org/json"
]
```

Change 1: added `https://allowed-domain.com` (scheme + host prefix). Result:

- 07_http_allowlist_bypass_at.tgsk → E_HTTP: send error for final URL `https://evil.com/malware`
- 07_http_path_traversal.tgsk → E_HTTP: send error for `https://allowed-domain.com/etc/passwd`

Observation: The bypass sample uses `https://allowed-domain.com@evil.com/malware`.

- Parser host = `evil.com`, but the allow check uses a prefix match on the full URL string when a scheme is present.
- Because the raw URL string literally starts with `https://allowed-domain.com@...`, the `starts_with` check passes, even though the actual host is `evil.com`.
- Outcome: allowlist admits the request; the runtime attempts the send, which then fails at network layer (DNS/TLS or connectivity) in this environment.

Change 2: added bare host `evil.com` to allowlist and appended `>[print]` to both breaker scripts for visibility:

- 07_http_allowlist_bypass_at.tgsk → still E_HTTP on send to `https://evil.com/malware`
- 07_http_path_traversal.tgsk → E_HTTP on send to `https://allowed-domain.com/etc/passwd`

Notes:
- The traversal path gets normalized to `/etc/passwd` on the remote host; since host allows, the request is attempted and then fails at send‑time here.
- If connectivity were available and the host returned non‑2xx, the handler would produce `E_HTTP_STATUS:<code>`; with 2xx and non‑JSON, it would output the text body; for JSON, it would output a JSON string.

## Hardened Allowlist (Implemented)

- Code change: `src/packets/execs/http.rs`
  - Compare parsed components instead of raw `starts_with`:
    - Require scheme match, exact host match (case-insensitive), optional port match
    - If pattern URL contains a non-root path, require target path to start with that prefix
  - Reject any target URL containing userinfo (`user@host`) with `E_BOX_VIOLATION`

Re-run after hardening:

- 07_http_allowlist_bypass_at.tgsk → `E_BOX_VIOLATION: url with userinfo not permitted` (blocked before send)
- 07_http_path_traversal.tgsk → still attempts request to `/etc/passwd` on allowed host and fails at network layer in this env

Notes:
- The userinfo trick no longer passes allowlist; defense-in-depth in place.
- Path traversal inside the same allowed host is a remote/server concern; allowlist’s job is origin gating, not validating remote paths.

## Summary of Protections Observed

- Depth, loop, and parse guards are enforced (recursion, loop overflow, YAML recursion limit, JSON EOF).
- Box rule blocks filesystem escapes by default; red root required for file and run operations.
- Yellow gating prevents exec/run until consent flags are set.
- Network is deny‑by‑default and allowlisted via `.tagspeak.toml`.

## Cautionary Finding (expected by breaker design)

- Allowlist check uses a prefix match on the full URL string when patterns include `http(s)://...`. For URLs with userinfo (`user@host`), a crafted string can start with an allowed prefix while resolving to a different host (`evil.com`).
- Mitigation direction (if/when desired): compare allow entries against the parsed components (scheme, host, optional port, path prefix) rather than string prefix of the raw URL.

## Reproduction Commands

- Baseline: `Push-Location examples; ../target/release/tagspeak_rs.exe run breakers/<file>.tgsk; Pop-Location`
- Consent: set `TAGSPEAK_ALLOW_YELLOW=1`, `TAGSPEAK_ALLOW_EXEC=1`, `TAGSPEAK_NONINTERACTIVE=1` before running.
