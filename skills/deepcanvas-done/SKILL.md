---
name: deepcanvas-done
description: Mark a DeepCanvas task as completed. Use when the user indicates they are finished with a task. Triggers include - English - "I'm done", "I finished", "mark this done", "complete this task", "mark DC-142 as done", "DC-142 is done". Turkish - "bitirdim", "tamamlandı", "DC-142'yi bitirdim", "bu görevi tamamla", "DC-142 tamam". With no explicit task code, this completes the active task (the last one pulled). With an explicit task code, completes that specific task.
argument-hint: [<task-code>]
allowed-tools: Bash(deep done*)
---

# DeepCanvas: Mark Task Done

If the user provided a task code in $ARGUMENTS, run:

!`deep done $ARGUMENTS --headless`

If no task code was provided (active task), run:

!`deep done --headless`

From the JSON output:
- On success (`ok: true`): confirm to the user with the task code and title that was completed.
- On error (`ok: false`):
  - `NoActiveTask` — tell user there's no active task; they need to pull a task first or pass a code explicitly
  - `Api` with 409 — task is already marked done
  - `NotAuthenticated` / `NoProjectBinding` — usual guidance
  - Other — show message verbatim

Do not run any other commands afterward unless the user asks.
