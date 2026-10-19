# Prompts for expanding parley's data with other models

Copy a prompt, replace the `<<…>>` parts, paste the answer into the right
section of `templates.txt` / `slots.txt`, then run the checklist at the end.

Rules that apply to every prompt:

- Generated **training** data goes above `== dev ==`.
- **Dev/test** sets must come from a *different* model or a fresh session that
  never saw the training templates (prompt D), or they stop measuring anything.
- Never paste dev/test lines into a training prompt as "examples to vary".

---

## A — more training templates for one intent

```text
You are writing training data for a tiny intent + slot parser inside a Linux
application launcher (like Spotlight/rofi). Users type short commands into a
search box. Write NEW ways to phrase the intent below.

Intent: <<timer | remind | open | search | volume>>
Meaning: <<one line, e.g. "start a countdown timer, optionally with an action when it ends">>
Allowed slots for this intent (use ONLY these, exactly as written, in braces):
<<timer: {DUR} {ACTION}  |  remind: {DUR} {TIME} {TEXT}  |  open: {APP}  |
  search: {QUERY}  |  volume: {PCT}>>
Slot meanings:
  {DUR}    a duration (20m, ten minutes, half an hour)
  {TIME}   a point in time (5pm, tomorrow 7am, noon)
  {TEXT}   free text, what to be reminded of
  {ACTION} what should happen when a timer ends (notify me, beep)
  {APP}    an application name
  {QUERY}  a web search query
  {PCT}    a volume level (30%, max, 50)

Format — one template per line, nothing else:
  <<intent>>: words {SLOT} words
Optional syntax you may use:
  [word]          optional word or phrase:   timer: [please] start a {DUR} timer
  [then {ACTION}] optional group with a slot
  punctuation is fine and is its own token:  remind: {TEXT}, at {TIME}

Requirements:
- 40 lines, all different from each other and from the existing ones below.
- Realistic launcher input: short (2–8 words), mostly lowercase, imperative,
  often terse ("tea timer {DUR}", "{DUR} timer pls"). Mix in some polite
  forms, a few longer natural sentences, and 3–4 lines with a typo in a
  non-slot word ("tiemr", "remidn").
- Vary word order, verbs, fillers, and where the slot sits (start/middle/end).
- Every line must contain the slot(s) that make the command usable
  (timer → {DUR}; remind → {TEXT} and {DUR} or {TIME}; open → {APP};
  search → {QUERY}; volume → {PCT}).
- No questions for timer/remind/open/volume ("how do I set a timer" is NOT a
  command). Questions are fine for search.
- Don't put words inside a slot's braces, don't invent slot names, no
  numbers next to {DUR}/{TIME}/{PCT} (they hold the numbers themselves).
- No explanations, no numbering, no markdown.

Existing templates for this intent (do not repeat or trivially reword them):
<<paste the current lines of this intent from the training section>>
```

---

## B — more "not a command" lines (hard negatives)

```text
You are writing NEGATIVE training data for a tiny command parser inside a
Linux application launcher. The parser knows these commands: timer, remind,
open (an app), search (the web), volume. Write things users type into the
launcher that are NOT one of these commands, so the parser learns to leave
them alone.

Format — one per line, nothing else:
  none: words

Write 60 lines, in these groups (about 10 each, no headers):
1. bare app or tool names, alone or with one word ("gimp", "file manager")
2. settings / system words ("bluetooth", "display scaling", "power profile")
3. calculator input ("3 * 7", "15% of 80", "sqrt(2)")
4. command words WITHOUT the needed detail ("timer", "set a reminder",
   "volume", "open", "search the web")
5. questions ABOUT commands ("how do timers work", "why is my volume low",
   "how do i set a reminder in gnome")
6. random launcher input: greetings, file names, single nouns, gibberish

Rules: no lines that are real, complete commands (no "timer 5m", no
"open firefox", no "volume 30"). Short, mostly lowercase. No explanations,
no numbering, no markdown. Don't repeat these existing lines:
<<paste the current none: lines from the training section>>
```

---

## C — more training values for one slot

```text
You are writing slot values for a tiny command parser in a Linux app
launcher. Write NEW values for the slot below.

Slot: <<DUR | TIME | TEXT | PCT | APP | QUERY | ACTION>>
Meaning: <<e.g. "a duration as people type it into a timer command">>

Format — exactly one line, values separated by " | ":
  <<SLOT>> = value | value | value
In numbers you may write # to mean "a random number 1–90" (e.g. "#m",
"# minutes", "#:30"); use it for most numeric values.

Requirements:
- 30 values, all different from each other and from the existing ones below.
- Realistic typing: digits and spelled-out numbers, short and long forms,
  common abbreviations, a few unusual-but-real forms.
  <<slot-specific hints, e.g. DUR: "1h30", "an hour and a half",
    "quarter of an hour"; TIME: "half past 5", "tomorrow at 9";
    TEXT: everyday tasks, 1–6 words; APP: real Linux app names;
    QUERY: realistic web searches; ACTION: what a timer should do at the end>>
- No "|" inside a value. No explanations, no markdown.

Existing values (do not repeat): <<paste the current SLOT = … line>>
```

---

## D — a fresh dev or test set (different model or fresh session!)

```text
You are writing EVALUATION data for a tiny command parser in a Linux
application launcher. You have NOT seen the training data, and that matters:
write phrasings the way real users would, not variations of anything given.

Intents and their slots:
  timer   {DUR} [ {ACTION} ]     start a countdown, optionally do something at the end
  remind  {TEXT} + {DUR}/{TIME}  remind me of something later
  open    {APP}                  open an application
  search  {QUERY}                search the web
  volume  {PCT}                  set the volume level
  none    (no slots)             launcher input that is NOT one of these commands

Part 1 — templates. Format, one per line:
  intent: words {SLOT} words
Write 10 per intent and 15 none lines. Make them natural and diverse:
terse launcher style, polite forms, unusual verbs, punctuation, 1–2 typos.
For none, include hard cases: questions about commands, command words
without details, app names alone, calculator input.

Part 2 — values. Format, one line per slot, values separated by " | ",
# = a random number:
  DUR<<?/!>> = …     (6 values)
  TIME<<?/!>> = …    (6)
  PCT<<?/!>> = …     (5)
  TEXT<<?/!>> = …    (6)
  APP<<?/!>> = …     (6 real Linux apps)
  QUERY<<?/!>> = …   (6)
  ACTION<<?/!>> = …  (5)
Use "?" after the slot name for a dev set, "!" for a test set.

No explanations, no markdown, just the lines.
```

---

## After pasting: checklist

```sh
# 1. no held-out phrasing may be producible from training
cargo run -p parley --features training --bin parley-train --release -- --leaks

# 2. format, balance, value leaks
cargo test -p parley --features training

# 3. retrain + judge on dev (look at errors here)
cargo run -p parley --features training --bin parley-train --release -- --errors 20

# 4. occasionally: the honest numbers (never fix things because of test errors)
cargo run -p parley --features training --bin parley-eval --release
```

If `--leaks` lists something: drop the training line, or remove the list word
that produces it. Only reword the dev/test line when the training line is too
general to remove.

Read generated lines before keeping them. Models like to drift: questions
labelled as commands, slots that don't fit the intent, numbers glued next to
`{DUR}`, and near-duplicates.
