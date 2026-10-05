# winotch

A small always-on-top "notch" at the edge of your screen that shows what is going on, starting with the live activity of [Claude Code](https://claude.com/claude-code): working, waiting for a permission, waiting for your answer, done, error.

Windows first; macOS and Linux are planned.

## Features

- **Notch**: borderless, always on top, never steals focus, clicks pass through outside its shape. Attach it to any screen edge, on any monitor, and slide it along the edge. Hidden while an app is fullscreen.
- **Expands** on hover and when something needs your attention, with a short synthesized sound.
- **Cursor resistance** (Windows): the cursor stops at the notch edge until you push through.
- **Modules**: everything the notch shows comes from modules, each with its own on/off switch and settings.
  - **Claude Code**: one dot per session, colored by state; click the notch to acknowledge finished sessions.
  - **Date and time** (off by default).

## How it works

winotch registers HTTP hooks in your Claude Code settings (`~/.claude/settings.json`), from the settings window. Claude Code then posts its events to a small server that winotch runs on `127.0.0.1`, protected by a per-install token. Nothing leaves your machine.

## Build from source

Requirements: [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) and [pnpm](https://pnpm.io/), plus the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```sh
pnpm install
pnpm tauri dev      # run with hot reload
pnpm tauri build    # build the installers (Windows: NSIS and MSI)
```

Checks run by the CI:

```sh
pnpm check                                          # front-end types
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Project layout

- `src/`: front-end (Svelte 5 + TypeScript), display only.
- `src-tauri/src/`: backend (Rust, Tauri 2): notch window, placement, settings, and the module contract (`module.rs`).
- `src-tauri/src/modules/<module>/` and `src/modules/<module>/`: one folder per module, Rust side and settings side.

## License

[GPL-3.0](LICENSE)
