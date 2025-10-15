## TagSpeak Error Styling Cheat‑Sheet

Use these scenarios to sanity‑check the current parser/UI error handling. All examples assume you run `tagspeak_rs path/to/script.tgsk` and expect the colourful panel to appear (plus the prefix line noted below).

### 1. Unexpected Character (missing opening bracket)

```tgsk
print@"hello"]
```

- **Prefix line:** `Malformed packet starting on line …`
- **Panel detail:** `unexpected character at top-level: 'p'`
- **Hint:** “Packets begin with '[' — let's tuck one in right before this darling.”

### 2. Empty Packet Op (`[]`)

```tgsk
[print@"hi"]>[]   # dangling [] chain
```

- **Prefix line:** `Empty packet op on line …`
- **Panel detail:** `engine says: empty packet op in []`
- **Hint:** “Did you forget to add in an argument? Arguing with [] is half the fun. :D”

### 3. Unterminated String

```tgsk
[print@"hello]        # missing closing quote
```

- **Prefix line:** `Malformed …`
- **Panel detail:** `engine says: unterminated string starting before …`
- **Hint:** “Seal that quote with '"' before it floats away. <3”

### 4. Unbalanced Brackets

```tgsk
[print@"hi"          # no closing ]
```

- **Prefix line:** `Malformed …`
- **Panel detail:** `engine says: unbalanced [ ... ] before …`
- **Hint:** “A matching ']' would make this perfect. <3”

### 5. Extra Closing `]`

```tgsk
[print@"hi"]]
```

- **Prefix line:** `Malformed Print packet on line …`
- **Panel detail:** `extra closing ']' detected at …`
- **Hint:** “Looks like you have a typo here. It's ok. Happens to me also. <3”

### 6. `if` without condition

```tgsk
[if]{ [then]{ [print@"oops"] } }
```

- **Prefix line:** `Malformed packet starting on line …`
- **Panel detail:** `if needs (cond) or @(cond)`
- **Hint:** falls back to the generic “Something feels off—let's peek at those brackets together.”

### 7. Missing `[then]` in conditional chain

```tgsk
[if(x==y)]{ [print@"hi"] }
```

- **Prefix line:** `Malformed packet starting on line …`
- **Panel detail:** `expected [then]`
- **Hint:** generic fallback.

### 8. Unexpected Top‑level `]`

```tgsk
] [print@"hi"]
```

- **Prefix line:** `Malformed …`
- **Panel detail:** `unexpected character at top-level: ']'`
- **Hint:** `unexpected_hint(']', …)` (“Found a ']' without its partner…”)

### 9. Unterminated block `{ …`

```tgsk
[log@out.json]{
  [key(name)@"Charlie"
```

- **Prefix line:** `Malformed …`
- **Panel detail:** `unbalanced { ... } before …`
- **Hint:** generic fallback.

### 10. Generic Parser Panic

If a new parser error string doesn’t match any specialised hint, it still routes through the panel:

```tgsk
[foo(bar)]         # unknown packet op
```

- **Prefix line:** `Malformed packet starting on line …`
- **Panel detail:** the raw `engine says: …` text from the parser.
- **Hint:** generic fallback (“Something feels off…”).

> Tip: every panel line is colourised with ANSI escape codes. If your terminal displays raw `\x1b[...]` sequences, enable virtual terminal processing (Windows) or run within a colour-capable terminal emulator.
\n### Quick Driver Script\n\nRun \examples/basics/error_test.tgsk\ and uncomment the scenario you want to trigger. The interpreter stops at the first active error and prints the panel above.
