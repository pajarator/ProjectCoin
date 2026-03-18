---
name: archiverun
description: Archive a completed RUN experiment to archive/RUNX/
disable-model-invocation: true
argument-hint: "<RUN number>"
---

Archive RUN$ARGUMENTS. Follow these steps exactly:

## 1. Create archive directory

```
mkdir -p archive/RUN$ARGUMENTS/
```

## 2. Identify and copy RUN files

Find all files belonging to this RUN:
- `coinclaw/src/run$ARGUMENTS.rs` — the Rust backtest script
- `run$ARGUMENTS_*_results.json` — results files in project root
- Any `run$ARGUMENTS_*.py` scripts in project root

Copy them into `archive/RUN$ARGUMENTS/`.

## 3. Write RUN$ARGUMENTS.md

Create `archive/RUN$ARGUMENTS/RUN$ARGUMENTS.md` documenting the experiment. Use existing archive entries (e.g. `archive/RUN29/RUN29.md`) as a template. Must include:
- **Goal** — what hypothesis was tested
- **Method** — grid parameters, data used, coin count, day count
- **Results** — full summary table with all grid combos
- **Per-coin breakdown** — diagnostics table for key coins
- **Conclusion** — POSITIVE / NEGATIVE / CONDITIONALLY POSITIVE, with reasoning
- **Files** — list of archived files

## 4. Clean up working files

- Delete `coinclaw/src/run$ARGUMENTS.rs`
- Delete `run$ARGUMENTS_*_results.json` from project root
- Revert `coinclaw/Cargo.toml`: remove the `run$ARGUMENTS` feature line
- Revert `coinclaw/src/main.rs`: remove the `mod run$ARGUMENTS` declaration and the `--run$ARGUMENTS` CLI block

## 5. Update CLAUDE.md

Add a one-line entry to the archive list in CLAUDE.md under `### Archive Structure`, following the existing format:
```
  RUN$ARGUMENTS/ — <short description>. Result: <POSITIVE|NEGATIVE|CONDITIONALLY POSITIVE> — <key finding>. <COINCLAW changes or "No COINCLAW changes">.
```
Insert it in numerical order.

## 6. Verify

- `cargo build --release` must still compile (no dangling module references)
- `archive/RUN$ARGUMENTS/` contains the script, results JSON, and RUN$ARGUMENTS.md
- No `run$ARGUMENTS` references remain in `coinclaw/Cargo.toml` or `coinclaw/src/main.rs`
