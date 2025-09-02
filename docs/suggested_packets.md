# 🚨 Red + REPL Packets (Codex Spec)

Agents in TagSpeak are layered constructs: they bundle control loops, input, and adapters into higher-level behaviors.
The following are **red-tier** packets — risky by nature, gated by ritual, and never implicit.

---

## 🛑 `[red]`

**Purpose**: Marks a session as *risk-enabled*. Unlocks dangerous or destructive behaviors that are otherwise inert.

* **Default**: off.

* **Scope**: session-wide.

* **Activation**: requires **manual ritual phrase**, human-typed:

  ```
  I acknowledge red mode. I accept the risk.
  ```

* **Rules**:

  * Cannot be macro’d, scripted, or auto-set.
  * Once acknowledged, `session.flag = red` until `/exit`.
  * Does **not** bypass `[yellow]` (aka `[confirm]`) — each side-effectful packet still requires per-action consent.
  * Unlocks red-only packets: `[repl]`, `[realtime]`, `[exec:*]`.

---

## 🟡 `[yellow]` / `[confirm]`

**Purpose**: Consent gate. Asks before executing its body.

* **Alias**: `[yellow@...]` and `[confirm@...]` are routed identically by runtime.
* **Syntax**:

  ```tgsk
  [yellow@"message"]{ [packet] }
  ```

  (block form requires a body or errors)
* **Behavior**:

  * Prints header: `[confirm] <message>` then prompts once: `Proceed? [y/N/a]`
  * `y/yes`: run once.
  * `a/always`: run and remember for rest of process.
  * default/N: skip, returns Unit.
  * Internally increments `__yellow_depth` so gated packets (e.g., `[exec]`) know they are inside a confirm.
* **Env overrides**:

  * `TAGSPEAK_ALLOW_YELLOW=1|true|yes|y`
  * Sugar-specific: `TAGSPEAK_ALLOW_EXEC`, `TAGSPEAK_ALLOW_RUN`
* **Non-interactive**: If `TAGSPEAK_NONINTERACTIVE=1|true`, prompts auto-deny (skip).
* **Interplay**:

  * `[exec]` requires yellow unless allowed by env or `.tagspeak.toml`.
  * `[run]` can be configured to require yellow (`require_yellow_run`) in `.tagspeak.toml`.

---

## 🔁 `[repl]`

**Purpose**: Opens a *read–eval–print loop* between TagSpeak and a bound adapter (usually an LLM).
This is the **first canonical red packet** — intentionally stupid and dangerous, because it lets a model talk through the same DSL designed to control it.

* **Scope**: one per TTY. Cannot nest.

* **Structure**:

  ```tgsk
  [red@"STOP SIGN: risky mode"]
  [repl:(ollama/gemma3)]{
    [loop@yes][until@/exit]{
      [input.line@"$> "]        # auto-yellow, human only
      > [chat.stream@"$in"]     # model call
      > [render@plain]          # prints output
    }
  }
  ```

* **Behavior**:

  * Wraps a continuous loop.
  * `[input]` must always come from the human operator.
  * Output is rendered plain unless transformed downstream.
  * Exits cleanly on `/exit`.

* **Why red?**

  * REPL = self-referential.
  * LLM in TagSpeak = recursive hazard.
  * Sugar packet only — no hidden magic. The danger is purely in enabling unbounded interaction.
