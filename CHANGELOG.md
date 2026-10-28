# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Sign-in with a Microsoft account; the refresh token is kept in the system credential store.
- Instances for any game version with Fabric or Quilt, with automatic Java and memory.
- Mod, resource pack and shader catalog from Modrinth and CurseForge, with categories,
  sorting, project pages and required dependencies added automatically.
- Mod updates, by hand or right before the game starts, and picking any older version.
- Warnings in the mod list when a mod needs another one or conflicts with one.
- Changing an instance's game version, with a check of which mods have a matching build.
- Modpack import and export (`.mrpack` and CurseForge `.zip`).
- Worlds, servers and screenshots tabs; joining a world or server straight from the launcher.
- Game console, crash analysis, play time and a button that stops a hung game.
- Search, sorting and groups on the home screen; copying an instance.
- Desktop shortcuts that start an instance.
- Skins page with a 3D preview, a skin library and cape selection; the player's head in the
  title bar.
- Discord status "Playing Selene Launcher".
- Launcher updates from GitHub Releases, checked against the SHA-256 digest GitHub publishes.
- Windows notifications, download speed and parallelism limits, custom Java path.
- Portable mode: a `portable.txt` next to the executable keeps all data beside it.
- Russian and English interface; light, dark, Mica and glass styles.
