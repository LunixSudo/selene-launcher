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
- A third-party licenses page in the settings, generated at build time from the Rust and npm
  packages the launcher ships.
- Changing an instance's mod loader build, newer or older, next to its game version; the
  recommended build is marked in every list.
- An "Uninstall Selene Launcher" button at the bottom of the settings, which opens the Windows
  uninstaller. The portable version does not show it.
- Forge and NeoForge instances, including Forge and NeoForge modpacks. Their official installer
  runs in the background without windows, with its progress on the instance card; Forge
  suggests its recommended build. Forge goes back to 1.12.2 and NeoForge to 1.20.1.
- A new or imported instance downloads its game, Java and mod loader right away when an
  account is signed in, so the first start is quick.
- Required dependencies of another kind are installed too, for example a mod that a resource
  pack or shader needs goes into `mods`.
- A "Show beta builds" switch under the mod loader build list; betas are hidden by default.
- When Mojang, Fabric or Forge cannot be reached, downloads move on to the official mirrors
  those projects list. Forge and NeoForge libraries are fetched this way before their
  installer runs.

### Changed

- Starting an installed game no longer hashes every asset and library again: files that are
  in place with the right size are trusted, since they were checked when downloaded.
- The game starts without an internet connection once it has been installed: the version
  list, version files, loader profiles and the Java list are kept on disk, and a signed-in
  account plays single player offline as in the official launcher.
- Loader profiles are saved with their library hashes, which spares a dozen requests per launch.
- The add-ons list hashes files in parallel and remembers hashes of files that did not change.
- The game console receives output in batches instead of one message per line.
- Mods that CurseForge does not allow to download are looked up in one request.
- The installer puts the program into `%LOCALAPPDATA%\SeleneLauncher`, without a space, and
  moves an install from the old `Selene Launcher` folder there.
- The privacy policy mentions that catalog pages load images from the sites their authors
  chose.
- New logo: a moon-rock cube with a golden crescent, with a simpler drawing for
  16–32 px icons. Sources are in `docs/brand`.
- The solid style is the default, so the launcher stays light on slower PCs.
- An imported modpack shows up in the list right away, while its files download.
- Fabric and Quilt say when they have no build for an old game version instead of reporting
  a connection error.
- The catalog no longer has a separate select mode.
- Alpha and beta versions of the game can be picked when creating an instance, and versions
  before 1.7.3 get their sounds and textures laid out the way they expect.
- Loader builds are listed newest first, and Fabric builds are no longer all marked as betas.
- Desktop shortcut icons carry an image for every size Explorer shows, so their rounded corners
  stay smooth.
- Mica and glass stay on while the launcher window is inactive, for example under a file dialog.
- The settings page uses the full window width. Scrolling areas fade out at their edges instead
  of cutting content off, and scrollbars are thin and rounded instead of the Windows ones.
- Launcher data lives in `%APPDATA%\SeleneLauncher` instead of a folder named after the app
  identifier.
- A data pack can only be added once the instance has a world, and the message says so.

### Fixed

- A game log line that is not UTF-8 (the Windows console code page) no longer stops the log
  from being read, which could freeze the game once its output pipe filled up.
- Two downloads of the same file in one batch no longer write over each other.
- Instance ids and file names coming from the interface can no longer point outside the
  launcher's folders.
- Starting a game no longer undoes settings changed while it was installing.
- An expired or revoked sign-in now says to sign in again instead of blaming the server.
- A launcher update waits until no game is running or installing, and its errors are shown.
- Accounts and settings are written to a temporary file first, so a crash cannot leave them
  half-written.
- Loader libraries with natives for several platforms no longer replace one another.
- Dragging a link out of the window no longer drops a web shortcut on the desktop.
- The focus ring of search fields is no longer cut off at the edge of a scrolling area.
