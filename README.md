# commitai

> AI-generated commit messages for [lazygit](https://github.com/jesseduffield/lazygit) — with **any** AI provider.

`commitai` reads your staged git diff and asks an AI to suggest commit messages.
You pick one from a menu right inside lazygit.

A from-scratch Rust rewrite of [`chhoumann/bunnai`](https://github.com/chhoumann/bunnai),
with one big difference: **you are not locked to OpenAI**. Anything that speaks the
OpenAI-compatible API works — OpenAI, OpenRouter, Groq, Together, DeepSeek, and local
models via Ollama / LM Studio.

## Installation

Pick whatever fits your setup. All paths install a single static binary called
`commitai` — no runtime, no Node, no Python.

### Quick install (Linux, macOS, FreeBSD, WSL)

```sh
curl -fsSL https://raw.githubusercontent.com/brunosmessias/commitai/main/install.sh | sh
```

The script detects your OS/arch, downloads the right binary from the latest
GitHub Release, and installs it to `~/.local/bin` (or `/usr/local/bin` if you
run it with `sudo`).

### From source (requires [Rust](https://rustup.rs))

```sh
cargo install commitai
```

### Pre-built binaries

Grab the latest tarball for your platform from the
[Releases page](https://github.com/brunosmessias/commitai/releases/latest),
extract, and put `commitai` on your `PATH`.

### Homebrew (macOS / Linux)

```sh
brew install brunosmessias/tap/commitai
```

## Highlights

- 🚀 **Single static binary** (~3 MB), no runtime/Node/Bun to install
- 🔌 **Any provider** — just a base URL, API key, and model
- 🧙 **Interactive first-run wizard** with provider presets and a live connection test
- 📝 **Editable prompt templates** (multiple, switchable with `--template`)
- 🎛️ **`commitai config` menu** + a scriptable `config set` for automation
- 🔐 Keys stored locally in `~/.config/commitai/config.toml`, never sent anywhere but your provider
- 🪝 **Drop-in** for existing lazygit setups (same `1. <message>` output format)

## Installation

### From source (recommended for now)

```sh
git clone <this-repo> commitai
cd commitai
cargo install --path .
```

This puts `commitai` on your `PATH`. Requires the [Rust toolchain](https://rustup.rs).

### Pre-built binaries

Grab the latest from the Releases page and put it on your `PATH`.

## First-time setup

Just run:

```sh
commitai config
```

You'll get an interactive wizard:

1. **Pick a provider** — OpenAI, OpenRouter (gives access to GPT *and* Claude *and* Gemini),
   Groq, Together, DeepSeek, Ollama, LM Studio, or a custom URL.
2. **Paste your API key** (local providers skip this). The wizard shows you where to get one.
3. **Choose a model** — type one, or fetch the list straight from the provider.
4. **Test the connection** — it verifies reachability + auth in real time.

Config lives at `~/.config/commitai/config.toml`:

```toml
[provider]
base_url = "https://openrouter.ai/api/v1"
api_key = "sk-..."
model = "anthropic/claude-3.5-sonnet"

[templates]
default = "/home/you/.config/commitai/templates/default.txt"
```

### Non-interactive configuration

```sh
commitai config set base_url https://api.groq.com/openai/v1
commitai config set api_key gsk_...
commitai config set model llama-3.3-70b-versatile
commitai config show   # prints config (key masked)
commitai path          # prints the config file path
```

## Usage with lazygit

Add a custom command to your [lazygit](https://github.com/jesseduffield/lazygit) config:

### As a menu (pick & commit)

```yaml
customCommands:
  - key: "<c-a>" # ctrl + a
    description: "pick AI commit"
    command: 'git commit -m "{{.Form.Msg}}"'
    context: "files"
    prompts:
      - type: "menuFromCommand"
        title: "AI Commits"
        key: "Msg"
        command: "commitai"
        filter: '^(?P<number>\d+)\.\s(?P<message>.+)$'
        valueFormat: "{{ .message }}"
        labelFormat: "{{ .number }}: {{ .message | green }}"
```

### With vim (edit before committing)

```yaml
customCommands:
  - key: "<c-a>"
    description: "Pick AI commit"
    command: 'echo "{{.Form.Msg}}" > .git/COMMIT_EDITMSG && vim .git/COMMIT_EDITMSG && [ -s .git/COMMIT_EDITMSG ] && git commit -F .git/COMMIT_EDITMSG || echo "Commit message is empty, commit aborted."'
    context: "files"
    subprocess: true
    prompts:
      - type: "menuFromCommand"
        title: "AI Commits"
        key: "Msg"
        command: "commitai"
        filter: '^(?P<number>\d+)\.\s(?P<message>.+)$'
        valueFormat: "{{ .message }}"
        labelFormat: "{{ .number }}: {{ .message | green }}"
```

Then stage changes (`c` to stage files in lazygit) and press `Ctrl+A`.

### Why the menu never lies to you

`commitai` does not blindly pipe the model's reply to lazygit. Most chat
models will happily prepend a `<think>` block, add a "Here are the
suggestions:" preamble, or tack on a follow-up offer — any of which would
otherwise show up as a phantom item in the lazygit menu.

The pipeline enforces this contract on both sides:

1. The **system prompt** tells the model, in no uncertain terms, to reply
   with *exactly* N lines of the form `N. <message>` and nothing else.
2. The **output cleaner** in `src/output.rs` re-parses the model's reply,
   extracts only the lines that actually look like `N. <message>`, drops
   duplicates, drops empty lines, and caps the result at N.
3. The final stdout is **always** at most N numbered lines — never more, so
   the lazygit menu never has stale or empty entries.

The cleaner is also tested against tricky inputs (dates like
`2024. 01. 15`, decimals like `1.5 cups`, paren-style `1) foo`) to make
sure junk doesn't sneak through.

## Choosing how many suggestions to show

```sh
commitai --n 5         # 5 options in the menu (default: 3)
commitai --format conventional   # use conventional commits instead of gitmoji
commitai --format custom         # follow your user template's instructions
```

`--n` and `--format` are also persisted:

```sh
commitai config set format gitmoji
```

## Custom prompt templates

```sh
commitai config   # → "Prompt templates" → "+ Add new template"
```

Templates are plain text files containing a `{{diff}}` placeholder. Use a specific one at runtime:

```sh
commitai --template my-template
```

## Using local models (no API key)

Pick **Ollama** or **LM Studio** in the wizard (or `config set base_url http://localhost:11434/v1`).
With a local base URL the key check is skipped.

## CLI reference

```
commitai                         Generate commit messages from staged changes
commitai --template <name>       Use a specific prompt template
commitai --verbose               Print debug info
commitai config                  Interactive config menu / first-run wizard
commitai config show             Print current config (key masked)
commitai config set <key> <val>  Set base_url | api_key | model
commitai path                    Print the config file path
```

## Credits

Inspired by and a rewrite of [`chhoumann/bunnai`](https://github.com/chhoumann/bunnai).

## License

MIT
