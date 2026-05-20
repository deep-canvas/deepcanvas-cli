---
name: deepcanvas-pull
description: Fetch the full context for a DeepCanvas task (description, acceptance criteria, linked PRDs) and read it into the session. Use this when the user wants to start working on a specific task. Triggers include - English - "let's work on DC-142", "pull task DC-142", "start working on DC-142", "I'll do DC-142", "fetch context for DC-142", "what's DC-142 about". Turkish - "DC-142'yi çekelim", "DC-142 üzerinde çalışalım", "DC-142'ye başlayalım", "DC-142 ne hakkında", "DC-142 dokümanını al". Also use after the user picks a task from a list provided by deepcanvas-tasks. Task codes follow the pattern PREFIX-NUMBER (e.g., DC-142, ENG-7, AUTH-23).
argument-hint: <task-code>
allowed-tools: Bash(deep pull*), Read(.deep/**)
---

# DeepCanvas: Pull Task Context

The user wants to work on task $ARGUMENTS. Fetch its full context.

1. Run:
   !`deep pull $ARGUMENTS --headless`

2. From the JSON output, locate the file paths:
   - `results[0].files.task_md` — the task description with acceptance criteria
   - `results[0].files.documents[].path` — linked PRD documents

3. Read each file using the Read tool:
   - First the `task_md`
   - Then the primary document (where `primary: true`)
   - Then any related documents

4. Summarize for the user:
   - Task title and short description (2-3 sentences)
   - Acceptance criteria as a checklist
   - Key points from the primary PRD that are relevant to implementation
   - Your initial 3-5 step implementation plan as a numbered list

5. Ask if they want to proceed with the plan before writing any code.

If the deep pull command errors:
- `NotAuthenticated` — tell user to run `deep login`
- `NoProjectBinding` — tell user to run `deep init` first
- `TaskNotFound` or `Api` with 404 — task code doesn't exist in this project
- `InvalidTaskCode` — code format wrong (must be PREFIX-NUMBER)
- Other — show message verbatim

Do not start implementing until the user confirms the plan.
