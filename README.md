# deep — DeepCanvas CLI

Task and document context for coding agents.

## Install

```bash
curl -fsSL cli.deepcanvas.studio/install.sh | sh
```

Or via Homebrew:

```bash
brew install deepcanvas-studio/tap/deep
```

## Usage

```bash
deep login                                # authenticate this device
deep init <org-slug>/<project-slug>       # bind current directory to a project
deep tasks                                # list assigned tasks
deep pull DC-142 [DC-156 ...]             # fetch task context into .deep/<code>/
deep completion bash                      # generate shell completion
deep update                               # self-update from GitHub releases
deep logout                               # remove local credentials
```

`deep init` writes `.deep/config.toml` and updates `.gitignore` to ignore
per-task cache directories. The config file should be committed.

## Environment

| Variable | Default |
|---|---|
| `DEEPCANVAS_API_URL` | `https://api0910.deepcanvas.studio` |
| `DEEPCANVAS_FRONTEND_URL` | `https://app.deepcanvas.studio` |
| `DEEP_VERSION` | (used by `install.sh` to pin a version) |
| `NO_COLOR` | disables colored output |

## Build from source

```bash
cargo build --release
./target/release/deep --help
```

## License

MIT — see [LICENSE](./LICENSE).
