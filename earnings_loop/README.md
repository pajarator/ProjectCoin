# Earnings Loop — Ralph Loop for COINCLAW

An automated discovery loop that iteratively generates earnings-trading hypotheses for COINCLAW.

## Quick Start

```
/ralph-loop
```

When prompted, paste the contents of `earnings_loop/PROMPT.md` as the prompt.
Use `--max-iterations 100` to run up to 100 cycles.

## What Each Iteration Does

1. **Reads state** from `earnings_loop/state.json`
2. **Scans archive** to avoid duplicate ideas from prior RUNs
3. **Reads COINCLAW source** to understand tweakable parameters
4. **Brainstorms ONE earnings-trading hypothesis** not yet tried
5. **Writes** `archive/RUNx/RUN_x_suggestion.md`
6. **Updates state** (increments cycle count, marks RUN complete, advances `next_run`)
7. **Outputs** `<promise>RUN_X_COMPLETE</promise>` only after file is on disk

## Directory Structure

```
earnings_loop/
  PROMPT.md       # Ralph Loop instructions (paste this into /ralph-loop)
  state.json      # Run counter, cycle tracker, completed RUNs
  README.md       # This file

archive/
  RUN38/          # Created on first iteration
    RUN38_suggestion.md
  RUN39/
    RUN39_suggestion.md
  ...             # One directory per completed iteration
```

## Checking Progress

```bash
# View state
cat earnings_loop/state.json

# List completed suggestions
ls archive/RUN*/

# Read a suggestion
cat archive/RUN38/RUN38_suggestion.md
```

## Cancelling

```
/ralph-loop:cancel-ralph
```

Or press `Ctrl+C` — the loop handles SIGINT gracefully. State is preserved.

## Max Cycles

Default is 100 iterations. Each iteration is independent — the loop picks up from `next_run` in `state.json` on subsequent invocations.

## What Makes a Good Hypothesis

A valid `RUN_x_suggestion.md` includes:
- Named hypothesis with clear mechanism
- Specific COINCLAW config parameter changes (concrete values)
- Expected outcome and validation method
- Out-of-sample testing approach
- Why it's not a duplicate of prior RUNs
