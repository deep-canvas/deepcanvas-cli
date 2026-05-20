---
name: deepcanvas-tasks
description: List the user's assigned DeepCanvas tasks for the current project. Use this whenever the user asks to see their tasks, work items, todos, or what they should work on. Recognize these patterns in any language - English examples - "what are my tasks", "show me my tasks", "what should I work on today", "list my work", "what's on my plate", "what do I have to do". Turkish examples - "bugünkü görevlerimi göster", "görevlerimi listele", "bugün ne yapmalıyım", "görevlerim neler", "bana görevlerimi göster". Only triggers within a directory linked via `deep init`.
allowed-tools: Bash(deep tasks*)
---

# DeepCanvas: List Tasks

Run the DeepCanvas CLI in headless mode:

!`deep tasks --headless`

Parse the JSON output and present the tasks to the user as a readable list or table. Include the task code, title, status, energy level, and linked PRD code.

If the user picks a task from this list (e.g., "let's work on DC-142", "I'll do the first one"), invoke the `deepcanvas-pull` skill with that task code.

If the output is an error (non-zero exit, `ok: false` in JSON), interpret the `error.kind` field:
- `NotAuthenticated` — tell the user to run `deep login` in their terminal
- `NoProjectBinding` — tell the user to run `deep init` to link this directory to a DeepCanvas project
- `Unauthorized` — token may have been revoked; user should run `deep login` again
- Other — show the message verbatim

Do not run any other commands. Do not modify any files. This skill is read-only.
