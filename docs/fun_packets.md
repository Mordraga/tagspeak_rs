# 🃏 TagSpeak Fun Packets

A collection of joke, mood, and gremlin-coded packets designed to bring joy, sass, and occasional destruction to your TagSpeak scripts.

---

## ☠️ `[PWK]` — Power Word Kill

**Purpose:** Dramatic self-destruct. Countdown, ASCII skull, and exit. Each number should countdown by half a second before displaying the next.

**Example:**
```tgsk
[PWK]
```
**Output:**
```
5
4
3
2
1

  _____
 /     \
| () () |
 \  ^  /
  |||||
  |||||

💀  Execution terminated by Power Word Kill.
```

---

## 🔮 `[summon@thing]` — Ritual Failure

**Purpose:** Attempts to summon a thing. Always fails.

**Example:**
```tgsk
[summon@coffee]
```
**Output:**
```
You attempt to summon coffee... nothing happens.
```

---

## 🦎 `[gecko]` — Mascot Print

**Purpose:** Prints a ASCII banner for TagSpeak.

**Example:**
```tgsk
[gecko]
```
**Output:**
```
+===================================================+
| __ _____           ____                   _    __ |
|| _|_   _|_ _  __ _/ ___| _ __   ___  __ _| | _|_ ||
|| |  | |/ _` |/ _` \___ \| '_ \ / _ \/ _` | |/ /| ||
|| |  | | (_| | (_| |___) | |_) |  __/ (_| |   < | ||
|| |  |_|\__,_|\__, |____/| .__/ \___|\__,_|_|\_\| ||
||__|          |___/      |_|                   |__||
+===================================================+                                                                                                
```

---

## 🙏 `[please]` — Desperate Begging

**Purpose:** Returns a sarcastic message when begging your code to work.

**Example:**
```tgsk
[math@2+2] > [please] > [print]
```
**Output:**
```
"This isn't going to work just because you begged."
```

---

## 💋 `[please:selene]` — Flirty Approval

**Purpose:** Echoes the value with a flirt-coded blessing from Selene.

**Example:**
```tgsk
[math@2+2] > [please:selene] > [print]
```
**Output:**
```
Since you asked nicely~ 💋
```

---

## 🛐 `[Deity@name]` — Bless This Program

**Purpose:** Prints a blessing from a named deity.

**Example:**
```tgsk
[Deity@Hephaestus]
```
**Output:**
```
Hephaestus blesses this program.
```

Special cases:
- `Astarte` → "Astarte blesses this program. Prepare for war or love."
- `Set` → “Set blesses this program. Prepare for chaos.”

---

## 💀 `[Deadman]` — Post-Run Kill Switch

**Purpose:** Triggers a fatal message if not disarmed before script end.

**Example:**
```tgsk
[Deadman@"You forgot to disarm me."]
[...your logic...]
[disarm]
```

**Without disarm Output:**
```
☠️  DEADMAN SWITCH TRIGGERED
You forgot to disarm me.
```

---

## `[Alli]` - Wife Specific Packet.

**Purpose** Dev Wife Message

**Example**
```tgsk
[alli]
```

**Output**
```
Sarym is a smarty pants. ;p
```
---

**All packets are safe, boxed, and non-networked. Add them to your scripts for chaos, clarity, or pure gremlin joy.**

🦎