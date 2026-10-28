# Selene Launcher

A fast, minimal Minecraft: Java Edition launcher for licensed accounts.

- Sign in with a Microsoft account; game ownership is verified on every login
- Game versions, libraries and assets come straight from Mojang
- The right Java runtime is downloaded automatically for each version
- Mods, resource packs and shaders from Modrinth and CurseForge, with required dependencies
  resolved for you and updates that stay on the release channel you installed
- Worlds, servers and screenshots per instance, with one-click join through Quick Play
- Skins and capes with a 3D preview
- Portable mode: put an empty `portable.txt` next to the executable

There is no offline mode. You need a Minecraft: Java Edition license to play.

> Early development. Windows is the primary target for now.

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

See [CONTRIBUTING.md](CONTRIBUTING.md) for code style and commit conventions.

## License

[GPL-3.0-or-later](LICENSE)

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.
Minecraft is a trademark of Mojang AB.
