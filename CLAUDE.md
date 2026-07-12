# PatchedModrinth — fork of the Modrinth App (READ THIS FIRST)

This repository is **PatchedModrinth**, a personal fork of `modrinth/code` (the Modrinth monorepo — upstream's own docs begin at the `# Modrinth Monorepo` heading further down). It bakes a **Vencord-style plugin system**, an **in-app self-updater**, and **automatic upstream syncing** into the Modrinth desktop app (`apps/app` + `apps/app-frontend`).

- **Owner:** GitHub **LianJordaan**. **Fork repo:** `git@github.com:LianJordaan/PatchedModrinth.git` (public). **Upstream remote:** `https://github.com/modrinth/code.git`.
- **The user is a capable hobbyist, not a Rust/Vue expert — explain things clearly and don't assume deep framework knowledge.**
- The authoritative engineering reference is **[MAINTAINING.md](MAINTAINING.md)**; the user-facing overview is **[README.md](README.md)**. Keep all three in sync.

## North star

A "Vencord for the Modrinth App": a forked Modrinth desktop app (Tauri v2 + WebView2) with a built-in plugin system (toggle in-app: hide ads, run the same instance multiple times, custom CSS/themes) that:

- loads plugins on every launch with **no debug port and no background process**;
- ships as a **drop-in `Modrinth App.exe`** (no installer) that replaces an existing Modrinth install with **zero data loss** — same exe name + app identifier ⇒ same `%AppData%\ModrinthApp\` data dir;
- **stays synced with upstream automatically** and **updates itself in-app**.

## Working conventions (IMPORTANT — read before acting)

- **No local builds.** This machine has no C/C++ linker and no set-up Node/pnpm toolchain, so the Tauri app **cannot be built locally**. Everything builds in **GitHub Actions**. Never run `tauri build` / `cargo build` / `pnpm build` locally.
- **Don't install or run things on the user's PC without asking.** Git ops and writing files are fine; building/running the app or installing toolchains needs explicit OK.
- **`gh` CLI is NOT installed.** Use plain `git` (SSH auth works as LianJordaan) or the GitHub REST API via `curl`/Python (unauthenticated = 60 req/hr).
- **Keep the patch set thin** (new files + a few one-line hooks) so `git merge <upstream-tag>` rarely conflicts.
- **Ship carefully:** the proven loop is *write code → run a Workflow of review agents (compile / vue-tsc / logic / safety) → fix findings → commit → tag `v0.15.10-fork.N` → poll the CI run until it concludes*. Risky native code (exe swap, launcher changes) gets adversarially reviewed before tagging.
- **Indentation:** Rust files use **4 spaces** (rustfmt); frontend (JS/Vue) uses **tabs**. (The upstream "use TAB everywhere" note below is about the frontend.)

## Current status (latest release: v0.15.10-fork.7)

Everything works end-to-end and is shipping. Releases are standalone `Modrinth App.exe` files on GitHub Releases; the app updates itself in-app. Recent fork.N history: fork.2 switched releases to the raw exe (no installer); fork.3 fixed a leftover sidebar fade with ads hidden; fork.4 added the in-app updater and switched plugin JS to indirect eval; fork.5→6 made Multi-Launch a native Play-next-to-Stop button; fork.7 removed the core "already running" guard so relaunch actually works.

**Two optional follow-ups that need the user** (see MAINTAINING.md):
1. **Bundled Tauri auto-updater** — *not* enabled (needs a minisign signing key generated via the Tauri CLI). The custom in-app updater already covers updates, so this is optional.
2. **`RELEASE_PAT` repo secret** — lets `fork-sync` auto-*publish* releases (the merge + fork-tag already happen automatically without it; only the release build needs it, because the default `GITHUB_TOKEN` can't trigger another workflow).

## What's built — feature map

**Plugin system (Rust `addons` Tauri plugin):** `apps/app/src/api/addons.rs` — commands `read_plugins`, `set_plugin_enabled`, `get_plugins_dir`, `fork_apply_update`. Registered via one-liners in `apps/app/src/api/mod.rs`, `main.rs`, `build.rs`, and `capabilities/plugins.json` (`addons:default`). Plugins live in `%AppData%\ModrinthApp\plugins\<id>\` (`manifest.json` + `.js`/`.css`); enabled state in `plugins\enabled.json` (separate, so updating a built-in never resets toggles). Built-ins are embedded via `include_str!` from `apps/app/src/api/builtin_plugins/` and seeded on first run; `preserve: true` files (custom-css `user.css`) are never overwritten.

**Frontend loader:** `apps/app-frontend/src/plugins/plugin-loader.js` (run from `main.js` after mount) injects each enabled plugin's CSS as a `<style>` and runs its JS via **indirect eval** (`script-src 'unsafe-eval'` — a Tauri CSP nonce can't neutralize eval, unlike inline/blob scripts). `apps/app-frontend/src/plugins/plugin-state.js` holds a reactive `enabledPluginIds` set (populated by the loader, updated live on toggle) that native features read to gate themselves. **Settings → Plugins** tab: `apps/app-frontend/src/components/ui/settings/PluginsSettings.vue` + one entry in `AppSettingsModal.vue`.

**Built-in plugins:**
- **Hide Ads** (on by default) — CSS hides `.ad-parent`, the Modrinth+ upsell (`a[href="https://modrinth.plus?app"]`), and the sidebar fade (`.app-sidebar::after`); PLUS a native guard in `apps/app/src/api/ads.rs` `init_ads_window` that returns early when enabled, so the native ad webview is never created (`crate::api::addons::is_plugin_enabled("hide-ads")`, cfg'd `not(target_os = "linux")`).
- **Multi-Launch** (opt-in, native — manifest-only, no JS) — when `playing && enabledPluginIds.has('multi-launch')`, `apps/app-frontend/src/pages/instance/Index.vue` shows a native **Play** button next to **Stop** to launch another copy. The core dedupe guard in `packages/app-lib/src/launcher/mod.rs` (which rejected launching an already-running instance) was **removed**; accidental double-fire is prevented by disabling Play while a launch is in flight.
- **Custom CSS** (opt-in) — loads user CSS from `plugins\custom-css\user.css`.

**In-app self-updater:** `fork_apply_update(app, download_url, expected_sha256)` in `addons.rs`. Validates the URL (HTTPS + GitHub hosts only), downloads via `tauri_plugin_http::reqwest`, verifies size + `MZ` header + GitHub's published SHA-256 digest, then swaps (rename running exe → `Modrinth App.old.exe`, move new exe in with a copy-fallback rollback so the app is never left without an exe), `app.restart()`. Startup cleans up `*.old.exe`. Frontend: the "App updates" section of `PluginsSettings.vue`. Startup version banner: `apps/app-frontend/src/plugins/update-check.js`.

**CI:** `.github/workflows/fork-build.yml` (validate — builds the standalone exe on push), `fork-release.yml` (build + publish the exe on a `v*` tag / dispatch), `fork-sync.yml` (daily; merges new upstream **releases** into `main`, tags `v<upstream>-fork`). All inherited Modrinth workflows were **deleted** (they need private Blacksmith runners / Modrinth secrets and just hang or fail on a fork).

## Migration safety — do NOT change

- `productName` = `"Modrinth App"` and bundle `identifier` = `"ModrinthApp"` (in `apps/app/tauri.conf.json`) — unchanged so the fork shares `%AppData%\ModrinthApp\` (instances/accounts/settings preserved). Never add data-dir cleanup.
- The updater endpoint in `apps/app/tauri-release.conf.json` points at the fork's releases — **never** repoint it to `launcher-files.modrinth.com` (that would auto-update the fork back into real Modrinth).

## Gotchas / hard-won lessons

- **CSP nonce:** Tauri injects a nonce that neutralizes `'unsafe-inline'` and can block `blob:` scripts, so plugin JS uses **indirect eval** (`'unsafe-eval'` is honored regardless). CSP is relaxed in `apps/app/tauri.conf.json` (`script-src`, `connect-src` + `https://api.github.com`).
- **The in-memory launch guard** in `launch_minecraft` is why multi-launch only worked after restarting the app (a fresh process list). Removed in fork.7.
- **GitHub renames release-asset spaces to dots:** the download is `Modrinth.App.exe` and must be renamed back to `Modrinth App.exe` before dropping it into `%LOCALAPPDATA%\Modrinth App\`.
- **Apply timing:** CSS/JS plugins apply on next launch (restart-to-apply); the native Multi-Launch applies live via the reactive set.
- **Project memory does NOT travel** with a directory move (it's keyed to the project path under `~/.claude`). This CLAUDE.md + MAINTAINING.md + README.md are the authoritative, self-contained record — keep them current.

---

# Modrinth Monorepo

This is the Modrinth monorepo — it contains all Modrinth projects, both frontend and backend. When entering a project, either to edit or analyse, you should read it's CLAUDE.md.

## Architecture

- **Monorepo tooling:** [Turborepo](https://turbo.build/) (`turbo.jsonc`) + [pnpm workspaces](https://pnpm.io/workspaces) (`pnpm-workspace.yaml`)
- **Frontend:** Vue 3 / Nuxt 3, Tailwind CSS v3
- **Backend:** Rust (Labrinth API), Postgres, Clickhouse
- **Indentation:** Use TAB everywhere, never spaces

### Apps (`apps/`)

| App               | Description                    |
| ----------------- | ------------------------------ |
| `frontend`        | Main Modrinth website (Nuxt 3) |
| `app-frontend`    | Desktop/app frontend (Vue 3)   |
| `app`             | Desktop/app shell (Tauri)      |
| `app-playground`  | Testing playground for app     |
| `labrinth`        | Backend API service            |
| `daedalus_client` | Daedalus client implementation |
| `docs`            | Documentation site (Astro)     |

### Packages (`packages/`)

| Package            | Description                                           |
| ------------------ | ----------------------------------------------------- |
| `ui`               | Shared Vue component library (`@modrinth/ui`)         |
| `assets`           | Styling and auto-generated icons (`@modrinth/assets`) |
| `api-client`       | API client for Nuxt, Tauri, and Node/browser          |
| `app-lib`          | Shared app library                                    |
| `blog`             | Blog system and changelog data                        |
| `utils`            | Shared utility functions (mostly deprecated)          |
| `moderation`       | Moderation utilities                                  |
| `daedalus`         | Daedalus protocol                                     |
| `tooling-config`   | ESLint, Prettier, TypeScript configs                  |
| `ariadne`          | Analytics library                                     |
| `modrinth-log`     | Logging utilities                                     |
| `modrinth-maxmind` | MaxMind GeoIP                                         |
| `modrinth-util`    | General utilities                                     |
| `muralpay`         | Payment processing                                    |
| `path-util`        | Path utilities                                        |
| `sqlx-tracing`     | SQLx query tracing                                    |

## Pre-PR Commands

Run these from the **root** folder before opening a pull request - do not run these after each prompt the user gives you, only run when asked, ask the user a question if they want to run it if the user indicates that they are about to create a pull request.

- **Website:** `pnpm prepr:frontend:web`
- **App frontend:** `pnpm prepr:frontend:app`
- **Frontend libs:** `pnpm prepr:frontend:lib`
- **All frontend (app+web):** `pnpm prepr`
- **Labrinth (backend):** See `apps/labrinth/AGENTS.md`

The website and app `prepr` commands

## Dev Commands

- **Website:** `pnpm web:dev` (copy `.env` template in `apps/frontend/` first)
- **App:** `pnpm app:dev` (copy `.env` template in `packages/app-lib/` first)
- **Storybook (packages/ui):** `pnpm storybook`

## Project-Specific Instructions

Each project may have its own file with detailed instructions:

- [`apps/labrinth/AGENTS.md`](apps/labrinth/AGENTS.md) — Backend API
- [`apps/frontend/CLAUDE.md`](apps/frontend/CLAUDE.md) - Frontend Website

## Code Guidelines

### Comments
- DO NOT use "heading" comments like: `=== Helper methods ===`.
- Use doc comments, but avoid inline comments unless ABSOLUTELY necessary for clarity. Code should aim to be self documenting!

## Bash Guidelines

### Output handling
- DO NOT pipe output through `head`, `tail`, `less`, or `more`
- NEVER use `| head -n X` or `| tail -n X` to truncate output
- IMPORTANT: Run commands directly without pipes when possible
- IMPORTANT: If you need to limit output, use command-specific flags (e.g. `git log -n 10` instead of `git log | head -10`)
- ALWAYS read the full output — never pipe through filters

### General
- Do not create new non-source code files (e.g. Bash scripts, SQL scripts) unless explicitly prompted to
- For Frontend, when doing lint checks, only use the `prepr` commands, do not use `typecheck` or `tsc` etc.
- Types in `@modrinth/utils` are considered highly outdated, if a component needs them, check if you can switch said component to use types from `packages/api-client`
- When provided problems, do not say "I didn't introduce these problems" (shifting the blame/effort) - just fix them.

## Edit Tool - Whitespace Handling (CLAUDE ONLY)

The Read tool uses `→` to mark where line numbers end and file content begins.

**Rule:** Copy the EXACT whitespace that appears after the `→` marker.
- Whatever appears between `→` and the code text is what's actually in the file
- That whitespace must be used EXACTLY in Edit tool's old_string
- Don't count arrows, don't interpret - just copy what's after the `→`

**Example:**
14→		private byte tag;
For Edit, use: `		private byte tag;` (copy everything after →, including the two tabs)

**If Edit fails:** Stop and explain the problem. Do not attempt sed/awk/bash workarounds.

**IMPORTANT**: Trust the Read tool output. Copy what's after `→` into Edit immediately. DO NOT verify with sed/od/grep first - that's wasting time and the instructions already tell you to stop if Edit fails, not to pre-verify.

## Standards

Standards available at the @standards/ folder.
