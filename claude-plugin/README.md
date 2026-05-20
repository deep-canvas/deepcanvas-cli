# DeepCanvas Claude Code Plugin

Three skills that let Claude Code drive the DeepCanvas CLI:

- `deepcanvas-tasks` — list your assigned tasks
- `deepcanvas-pull` — fetch a task's description and PRDs into your workspace
- `deepcanvas-done` — mark a task complete

## Prerequisites

1. Install the DeepCanvas CLI:
   ```bash
   curl -fsSL cli.deepcanvas.studio/install.sh | sh
   ```

2. Authenticate and link your project:
   ```bash
   deep login
   deep init
   ```

## Install Plugin

In Claude Code:

```
/plugin marketplace add deep-canvas/deepcanvas-cli
/plugin install deepcanvas
```

## Usage

Just ask Claude naturally:

- "What are my tasks?" → lists assigned tasks
- "Let's work on DC-142" → pulls task context, reads PRD, suggests plan
- "I'm done" → marks the active task complete

All commands run with `--headless` and exchange JSON.

## How It Works

The plugin contains skills that map natural language to `deep` CLI commands. Skills are auto-triggered by Claude based on what you ask — no slash commands needed.
