<p align="center"><img src="docs/brand/logo.svg" width="160" alt=""></p>

<h1 align="center">Selene Launcher</h1>

<p align="center">
  A fast, good-looking Minecraft: Java Edition launcher for licensed accounts.<br>
  <a href="https://selene.games">selene.games</a> · <a href="README.ru.md">На русском</a>
</p>

<p align="center">
  <a href="https://github.com/LunixSudo/selene-launcher/actions/workflows/build.yml"><img src="https://github.com/LunixSudo/selene-launcher/actions/workflows/build.yml/badge.svg?branch=dev" alt="Build"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-blue" alt="License: GPL-3.0"></a>
</p>

You need a Minecraft: Java Edition license to play: you sign in with your Microsoft account,
and there is no cracked mode.

> Early development: there is no release yet. Windows is the main target, Linux builds are new.

## Why Selene

- It runs well on old PCs. Memory, Java and its flags are picked from your RAM, processor and
  mods, the game starts on the discrete graphics card, and the launcher gets out of the way
  while you play. The default style has no blur and no endless animations.
- Downloads keep working when a site is down. If Mojang, a loader site, Modrinth or
  CurseForge cannot be reached, files come from their official mirrors and then from public
  ones, still checked against the official checksums.
- Modrinth and CurseForge share one catalog. Dependencies are added for you, and modpack
  updates keep the add-ons you put in yourself.
- The launcher has no servers of its own, no analytics and no ads.

## Features

### Accounts

- Microsoft sign-in, with game ownership checked on every login and several accounts at once.
- A new account picks its player name right in the launcher; the name can be changed later,
  with a check that it is free and Mojang's 30-day limit shown up front.
- Single player without a connection for an account that has signed in before.

### Game

- Every version from the first alphas to the newest snapshots, with the right Java
  downloaded for each.
- Fabric, Quilt, Forge (from 1.7.10) and NeoForge. Forge and NeoForge run their own
  installer in the background, without windows.
- A new instance downloads everything right away, so the first start is quick, and an
  installed game starts without checking all its files again.
- Automatic memory: from 3 GB for vanilla up to 10 GB for big packs, at most half of the RAM.
  Games on Java 19 and newer keep their loaded classes and start faster the next time.
- Fallbacks when a site is down: the official mirrors of Mojang, Fabric and Forge, then
  [BMCLAPI](https://bmclapidoc.bangbang93.com/). Java falls back to Eclipse Temurin from
  [Adoptium](https://adoptium.net/) and to [Azul Zulu](https://www.azul.com/downloads/).

### Instances

- Search, sorting, groups, icons and covers, copies and desktop shortcuts drawn with the
  instance's own icon.
- Moving an instance to another game version, with its mods swapped for matching builds,
  and picking a newer or older loader build.
- Tuned Java arguments, or flags picked from a searchable list.
- Play time for each instance and in total on the home screen.

### Add-ons and modpacks

- Mods, resource packs, shaders and data packs from Modrinth and CurseForge in one tab, with
  several categories at once and required dependencies added for you.
- Updates on the release channel you installed, a warning before turning on something made
  for another version, and a mark on two copies of one mod.
- On Forge, OptiFine installs from its official site opened inside the launcher.
- Modpacks by game version, in the version you pick, updated later without losing your own
  add-ons. Import and export `.mrpack` and CurseForge `.zip`, or drop them on the window.

### Playing

- Worlds with renaming and backups, servers with joining in one click, and screenshots.
- A game console, crash analysis with advice and a button that stops a hung game.
- A Discord status that can be turned off.

### Skins

- An animated 3D preview, a library of saved skins and a row of capes to try on.

### Launcher

- A three-step guide on the first start: looks or speed (speed is suggested for older
  graphics), Microsoft sign-in and a first instance, plain or with FPS mods.
- Light and dark themes, Mica and glass styles, optional live backgrounds, animations and
  click sounds, a pixel font in the style of Minecraft, and style presets of your own.
- Russian and English, plus machine translations into Ukrainian, German, Spanish, Brazilian
  Portuguese, Polish, Turkish and Chinese, picked from the system language.
- Settings saved to a file and loaded on another PC.
- Updates from GitHub Releases checked against SHA-256, and "Report a problem" that fills in
  a bug report for you.
- Portable mode: an empty `portable.txt` next to the executable keeps all data beside it.

## Requirements

- Windows 10 or 11, 64-bit. The installer adds Microsoft Edge WebView2 if it is missing.
- Or Linux, 64-bit, with WebKitGTK 4.1.
- A Microsoft account that owns Minecraft: Java Edition. Java is downloaded by the launcher.

## Download

Official builds come only from this repository. The website [selene.games](https://selene.games)
links to the same files. Copies from other sites are not ours and may be changed.

Until the first release goes to
[GitHub Releases](https://github.com/LunixSudo/selene-launcher/releases), test builds are
attached to runs of the [Build](https://github.com/LunixSudo/selene-launcher/actions/workflows/build.yml)
workflow: a Windows installer and a portable `.zip`, and for Linux an AppImage and a `.deb`
package. The Windows installer is not signed yet, so SmartScreen may warn about it. The Linux
builds are not tested much yet and do not update themselves.

Every release comes with checksums and a signed statement that the files were built here
from the tagged commit; [SECURITY.md](SECURITY.md#checking-a-download) shows how to check them.

Data lives in `%APPDATA%\SeleneLauncher` on Windows and `~/.local/share/SeleneLauncher` on
Linux.

## Privacy

The launcher has no servers of its own and collects nothing. Your sign-in token is kept in
the system credential store and is only sent to Microsoft and Mojang. See
[PRIVACY.md](PRIVACY.md) ([на русском](PRIVACY.ru.md)) for every service the launcher talks
to and why. The installer shows the policy in the system language, and the launcher asks
again after an update that changes it.

## Building

Requirements: Rust (rustup installs the version pinned in `rust-toolchain.toml`), Node.js 26,
pnpm 10 and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
pnpm install
pnpm tauri dev
```

Release build:

```sh
pnpm tauri build
```

The CurseForge catalog needs an API key at build time in the `CURSEFORGE_API_KEY`
environment variable. Builds without it only search Modrinth.

`pnpm build` also writes `static/third-party-licenses.txt` from `cargo metadata` and the
installed npm packages, which the launcher shows under Settings → Third-party licenses, and
the installer's copies of the privacy policy in `src-tauri/nsis/privacy`.

## Project layout

- `crates/selene-core`: everything that does not need a window: Mojang, Microsoft sign-in,
  Modrinth, CurseForge, instances, mods, modpacks and launching the game.
- `src-tauri`: the desktop app. Tauri commands, the account store, Discord and the
  installer in `src-tauri/nsis`.
- `src`: the interface, written in SvelteKit and Svelte 5.

## Community

- [Discord](https://discord.gg/uaDvBeM8D2): help, ideas and chat.
- [Telegram](https://t.me/selenegames): release news.
- [YouTube](https://www.youtube.com/@selene-games): videos about the launcher.

## Contributing

Bug reports and ideas are welcome under
[Issues](https://github.com/LunixSudo/selene-launcher/issues); the launcher's "Report a
problem" fills one in for you. See [CONTRIBUTING.md](CONTRIBUTING.md) for code style and
commit conventions, [SECURITY.md](SECURITY.md) for reporting a vulnerability and
[CHANGELOG.md](CHANGELOG.md) for what changed.

## License

[GPL-3.0-or-later](LICENSE). The licenses of bundled libraries and fonts ship with the
launcher and are listed in its settings.

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.
Minecraft is a trademark of Mojang AB.
