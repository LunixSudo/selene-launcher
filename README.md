# Selene Launcher

A fast, minimal Minecraft: Java Edition launcher for licensed accounts.

There is no offline mode. You need a Minecraft: Java Edition license to play.

> Early development. Windows is the primary target for now.

## Features

- **Accounts.** Sign in with a Microsoft account; game ownership is verified on every login.
  Several accounts, with the player's head in the title bar.
- **Game.** Every version from the first alphas to the newest snapshots. Versions, libraries
  and assets come straight from Mojang, and the right Java
  runtime is downloaded for each version. Fabric, Quilt, Forge and NeoForge are supported;
  Forge and NeoForge run their own installer in the background, without any windows. A new
  instance downloads all of it right away, so the first start is quick.
- **Instances.** Search, sorting, groups, icons and covers, copies, desktop shortcuts drawn
  with the instance's own icon and cover, and
  moving an instance to another game version with its mods swapped for matching builds, and
  picking a newer or older mod loader build.
- **Add-ons.** Mods, resource packs, shaders and data packs from Modrinth and CurseForge in
  one tab. Filter the catalog by several categories at once, remove a project right from its
  catalog entry, and get required dependencies added for you. Updates stay on the release channel you
  installed, and the launcher warns before you turn on something made for another version.
  Data packs go into the world you pick.
- **Modpacks.** Import and export `.mrpack` and CurseForge `.zip` files, or drop them on the
  window.
- **Playing.** Worlds, servers and screenshots per instance, joining a world or server in one
  click, a game console, crash analysis, play time and a Discord status.
- **Skins.** A 3D preview, a library of saved skins and cape selection.
- **Launcher.** Updates from GitHub Releases checked against SHA-256, light and dark themes
  with Mica and glass styles, Russian and English, and a portable mode: put an empty
  `portable.txt` next to the executable.

## Download

There is no release yet. Until the first one goes to
[GitHub Releases](https://github.com/LunixSudo/selene-launcher/releases), test builds are
attached to runs of the [Build](https://github.com/LunixSudo/selene-launcher/actions/workflows/build.yml)
workflow: a Windows installer and a portable `.zip`. The installer is not signed yet, so
Windows SmartScreen may warn about it.

## Privacy

The launcher has no servers of its own and collects nothing. Your sign-in token is kept in
the Windows credential store and is only sent to Microsoft and Mojang. See
[PRIVACY.md](PRIVACY.md) ([на русском](PRIVACY.ru.md)) for every service the launcher talks
to and why. The installer shows the policy in the system language before installing, and the launcher
asks again after an update that changes it.

## Building

Requirements: Rust (stable), Node.js 22, pnpm 10 and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
pnpm install
pnpm tauri dev
```

Release build:

```sh
pnpm tauri build
```

The CurseForge catalog needs an API key at build time in the `CURSEFORGE_API_KEY`
environment variable. Release builds without it only search Modrinth.

## Project layout

- `crates/selene-core`: everything that does not need a window: Mojang, Microsoft sign-in,
  Modrinth, CurseForge, instances, mods, modpacks and launching the game.
- `src-tauri`: the desktop app. Tauri commands, the account store, Discord and the
  installer configuration in `src-tauri/nsis`.
- `src`: the interface, written in SvelteKit and Svelte 5.

See [CONTRIBUTING.md](CONTRIBUTING.md) for code style and commit conventions, and
[CHANGELOG.md](CHANGELOG.md) for what changed.

## License

[GPL-3.0-or-later](LICENSE)

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.
Minecraft is a trademark of Mojang AB.
