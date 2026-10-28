# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Sign-in with a Microsoft account; the refresh token is kept in the system credential store.
- Instances for any game version with Fabric or Quilt, with automatic Java and memory.
- Mod, resource pack, shader and data pack catalog from Modrinth and CurseForge, with
  several categories at once, sorting, project pages and required dependencies added
  automatically. Data packs go into a chosen world.
- An "Add-ons" tab that lists everything installed, with a warning before turning on
  something made for another game version.
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
- Privacy policy in English and Russian, shown in the installer in the system language and
  linked from the settings. After an update that changes it, the launcher asks to accept it
  again.
- Desktop shortcuts of instances use the instance's icon and cover and follow later changes.
- Removing a project from its catalog entry, with a confirmation.
- Rounded tooltips in place of the system ones.
- Forge and NeoForge instances, including Forge and NeoForge modpacks. Their official installer
  runs in the background without windows, with its progress on the instance card; Forge
  suggests its recommended build. Forge goes back to 1.12.2 and NeoForge to 1.20.1.
- A new or imported instance downloads its game, Java and mod loader right away when an
  account is signed in, so the first start is quick.

### Changed

- The solid style is the default, so the launcher stays light on slower PCs.
- An imported modpack shows up in the list right away, while its files download.
- Fabric and Quilt say when they have no build for an old game version instead of reporting
  a connection error.
- The catalog no longer has a separate select mode.
- Desktop shortcut icons carry an image for every size Explorer shows, so their rounded corners
  stay smooth.
- Mica and glass stay on while the launcher window is inactive, for example under a file dialog.
