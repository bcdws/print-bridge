# AGENTS.md

TdPrint: a fork of [PrintBridge](https://github.com/vergil-lai/print-bridge), a local print agent (Tauri 2, Vue 3, Rust). `origin` = `bcdws/print-bridge`, `upstream` = `vergil-lai/print-bridge`. Ships Windows only. See `docs/tdprint.md` for customizations and releases; `README_en.md`, `docs/printbridge-technical*.md` and `openwiki/` describe upstream.

## Fork rules

- Keep upstream merges cheap: no large Rust refactors, renames or reformatting of upstream files.
- Backend changes go in `tdprint*.rs`, with minimal call sites marked `// TdPrint：…`.
- Frontend is free to customize; prefer `tdprint.ts` or new files over `App.vue`, and hide features with `TDPRINT_SHOW_*` flags.
- Keep internal `print-bridge` names.
- Change `BUILTIN_ORIGINS` and `TDPRINT_BUILTIN_ORIGINS` together.
- Public repo: never commit the `TDPRINT_AUTH_TOKEN` value.
- Record new customizations in `docs/tdprint.md`.
- Commits: Conventional Commits, Chinese description.
- After merging `upstream/main`: keep fork versions, `README.md` and `tauri*.conf.json` settings (no `externalBin` on Windows), check `tdprint` call sites, limit new release-triggered workflows to `printbridge-v*` tags.

## Commands

```bash
pnpm --dir apps/desktop tauri dev         # Linux build
pnpm --dir apps/desktop typecheck         # also: lint, test, build
pnpm --dir apps/desktop exec vitest run src/api.test.ts -t "name"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p print-bridge-runtime --lib tdprint                # filter unit tests
cargo test -p print-bridge-desktop --test queue_tests <name>    # core/runtime integration tests
cargo xwin check --workspace --target x86_64-pc-windows-msvc    # compile-check Windows code

node --test scripts/*.test.mjs
```

Format only files you changed. Known failures (2026-11-26): `timeout_cancels_worker_before_late_pdf_write`, the Homebrew workflow script test, `format:check` on `CliIntegrationRow.vue`.

## Building

Windows 打包一律走 GitHub CI，**不要在本地打包**（本地不保留 Windows 构建产物，`target/` 下的 xwin 产物可随时删除）。Always pass `-R bcdws/print-bridge` to `gh` (it defaults to upstream).

```bash
# test installer, no release; builds pushed code only
gh workflow run build-windows.yml -R bcdws/print-bridge --ref <branch>

# official release
gh workflow run release-windows.yml -R bcdws/print-bridge -f bump=patch -f notes="更新说明"
```

- A release reaches all clients immediately: **ask the user first**. It runs no tests and pushes a version commit to `main`. Never bump versions by hand.
- Do not trigger other release/publish workflows or push a `release` branch.

## Architecture

`apps/desktop` → `crates/runtime` → `crates/cli` → `crates/core` (`apps/server` is not shipped).

- Printing: `server.rs` (WebSocket `/ws`, IP and Origin whitelists) or `remote_worker.rs` → `queue.rs` (serial worker) → `printing/` (Windows: SumatraPDF; Win32 spooler for raw).
- Management: `api.ts` → `commands.rs` → `CommandService` → `RuntimeCommandExecutor`. Keep logic out of the Tauri layer; register commands in `lib.rs`; sync `types.ts` with Rust by hand.
- The config directory in `core/src/config.rs` must match the identifier in `tauri.conf.json`.
