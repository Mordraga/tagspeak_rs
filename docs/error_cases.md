# TagSpeak Error Panel Reference

This file records a sampling of diagnostics and the copy we render inside the TagSpeak error panel. It doubles as a quick smoke-check whenever the error style is tweaked.

## Unexpected Character (Missing Opening Bracket)

```tgsk
print@"hello"]
```

- Prefix line: `Unexpected character near top-level on line …`
- Panel detail: `unexpected character at top-level: 'p'`
- Hint: `Syntax - Something’s a little out of place. Deep breath. Let’s sort out the syntax together.`

## Empty Packet Op (`[]`)

```tgsk
[print@"hi"]>[]
```

- Prefix line: `Empty packet op on line …`
- Panel detail: `engine says: empty packet op in []`
- Hint: `Packet - Looks like you've got an empty []. Add an op like [print@...] or remove it if it's not needed.`

## Unterminated String

```tgsk
[print@"hello]
```

- Prefix line: `Malformed …`
- Panel detail: `engine says: unterminated string …`
- Hint: `Delimiter - Opened a quote but didn’t close it. Pop in the missing '"' to finish the thought.`

## Unbalanced Brackets

```tgsk
[print@"hi"
```

- Prefix line: `Malformed …`
- Panel detail: `engine says: unbalanced [ ... ] …`
- Hint: `Delimiter - Started a packet and forgot the ']'. Let’s add the closing bracket to complete the pair.`

## Extra Closing `]`

```tgsk
[print@"hi"]]
```

- Prefix line: `Malformed …`
- Panel detail: `engine says: extra closing ']' detected`
- Hint: `Delimiter - Found an extra ']'. Remove it or pair it with an opener to balance things out.`

## `if` Without Condition

```tgsk
[if]{ [then]{ [print@"oops"] } }
```

- Prefix line: `Malformed packet starting on line …`
- Panel detail: `if needs (cond) or @(cond)`
- Hint: `Packet - That condition's missing a (cond). Try @(x > 0) or another valid expression.`

> Tip: run `tagspeak run examples/basics/error_test.tgsk` and uncomment any scenario to view the live panel output.
