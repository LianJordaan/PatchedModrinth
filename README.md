# PatchedModrinth

A personal fork of the [Modrinth App](https://modrinth.com/app) (`modrinth/code`) that bakes in a **Vencord-style plugin system** — toggle plugins in-app that hide ads, run the same Minecraft instance multiple times, or apply custom CSS/themes. Plugins load on every launch with **no debug port and no background process**.

- **Drop-in replacement.** Releases ship the standalone `Modrinth App.exe` (no installer). Close Modrinth, then replace the exe at `%LOCALAPPDATA%\Modrinth App\Modrinth App.exe` with the downloaded one — that's it. Your data (instances, accounts, settings, worlds) lives separately in `%AppData%\ModrinthApp\` and is **never touched**.
- **Stays current.** A scheduled workflow merges each new upstream Modrinth _release_ and publishes a fork build automatically.
- **Built-in plugins.** Hide Ads, Multi-Launch, and Custom CSS — manage them in **Settings → Plugins**.

Add your own plugin by dropping a folder into your plugins directory (open it from **Settings → Plugins → Open plugins folder**) containing a `manifest.json` plus its referenced `.js`/`.css`:

```json
{
	"id": "my-plugin",
	"name": "My Plugin",
	"description": "What it does.",
	"version": "1.0.0",
	"author": "you",
	"js": "index.js",
	"css": "styles.css",
	"enabledByDefault": false
}
```

See **[MAINTAINING.md](MAINTAINING.md)** for how the fork is built, released, kept in sync with upstream, and how the plugin system works.

> **Unofficial.** This is a personal build and is **not affiliated with or endorsed by Modrinth (Rinth, Inc.)**. All credit for the underlying app goes to the Modrinth team. Source is kept public per the upstream [COPYING.md](COPYING.md).

---

# ![Modrinth Monorepo Cover](/.github/assets/monorepo_cover.png)

![Issues](https://img.shields.io/github/issues-raw/Modrinth/code?color=c78aff&label=issues&style=for-the-badge)
![Pull Requests](https://img.shields.io/github/issues-pr-raw/Modrinth/code?color=c78aff&label=PRs&style=for-the-badge)
![Contributors](https://img.shields.io/github/contributors/Modrinth/code?color=c78aff&label=contributors&style=for-the-badge)
![Lines of Code](https://img.shields.io/endpoint?url=https://loctopus.creeperkatze.dev/github/modrinth/code/badge?style=flat&logoColor=white&color=c78aff&style=for-the-badge)
![Commit Activity](https://img.shields.io/github/commit-activity/m/Modrinth/code?color=c78aff&label=commits&style=for-the-badge)
![Last Commit](https://img.shields.io/github/last-commit/Modrinth/code?color=c78aff&label=last%20commit&style=for-the-badge)

## Modrinth Monorepo

Welcome to the Modrinth Monorepo, the primary codebase for the Modrinth web interface and app. It contains ![Lines of Code](https://img.shields.io/endpoint?url=https://loctopus.creeperkatze.dev/github/modrinth/code/badge%3Fformat%3Dhuman&logoColor=white&color=black&label=) lines of code and has ![Contributors](https://img.shields.io/github/contributors/Modrinth/code?color=black&label=) contributors!

If you're not a developer and you've stumbled upon this repository, you can access the web interface on the [Modrinth website](https://modrinth.com) and download the latest release of the app [here](https://modrinth.com/app).

## Development

This repository contains two primary packages. For detailed development information, please refer to their respective guides:

- [Website frontend](https://docs.modrinth.com/contributing/knossos/)
- [Desktop app](https://docs.modrinth.com/contributing/theseus/)

## Contributing

We welcome contributions! Before submitting any contributions, please read our [contributing guidelines](https://docs.modrinth.com/contributing/getting-started/).

If you plan to fork this repository for your own purposes, please review our [copying guidelines](COPYING.md).

## Security

If you discover a security vulnerability within our codebase, please follow our [responsible disclosure guidelines](https://modrinth.com/legal/security).

## Support

If you need help with the Modrinth web interface or app, please visit our [support page](https://support.modrinth.com). For general inquiries, you can also join our [Discord server](https://discord.modrinth.com).

## License

All packages in this repository are licensed under their respective licenses. Refer to the LICENSE file in each package for more information.
