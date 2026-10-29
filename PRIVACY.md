# Selene Launcher Privacy Policy

Effective October 29, 2026.

Selene Launcher is a free, open-source launcher for Minecraft: Java Edition. It has no servers of its own: its developers do not receive or store any data about you.

## What stays on your computer

- **Sign-in.** You sign in on a Microsoft page, and the launcher never sees your password. The token used to sign you in again is kept in the Windows credential store (Credential Manager), not in a plain file. Your player name, UUID and skin are saved in the launcher folder to show your accounts.
- **Game data.** Instances, worlds, mods, saved skins, logs, crash reports and settings live in `%APPDATA%\SeleneLauncher`, or next to the program in the portable version.

None of this is sent anywhere, except as described below.

## What the launcher connects to

| Service | Why | What is sent |
| --- | --- | --- |
| Microsoft and Xbox Live | Signing in | Sign-in tokens |
| Mojang and Minecraft Services | Checking you own the game, downloading the game and Java, skins and capes | Your Minecraft token, the skin you choose |
| Fabric, Quilt, Forge and NeoForge | Mod loaders: the list of builds and the loader itself, which Forge and NeoForge download with their own installer. When a site cannot be reached, the same files come from the official mirrors the project lists, for Forge that is CreeperHost | The game version |
| BMCLAPI (bmclapi2.bangbang93.com, run by bangbang93) | A spare copy of the game, Java and loader files, used only when the official servers and their mirrors cannot be reached. BMCLAPI hands each file out from one of its download nodes, run by volunteers and Chinese university mirrors (CERNET), which also see your IP address. Files are checked against the official checksums whenever those can be had | The address of the requested file |
| Modrinth and CurseForge | The catalog, installing and updating mods | Search queries, and checksums of installed files to recognise them and find updates |
| GitHub | Checking for launcher updates | Nothing beyond the request itself |
| Discord (the app on your computer) | The "Playing Selene Launcher" status | The game version and instance name. Can be turned off in settings |

Like any website, these services see your IP address and handle data under their own policies. The launcher sends them nothing beyond what a feature needs.

Project pages in the catalog show icons, descriptions and galleries chosen by the authors of the projects. Their images can be hosted on other sites, which then also see your IP address when the page is opened.

The launcher does not collect statistics or analytics, show ads, or sell or share data with anyone.

## Deleting your data

- "Sign out" in the account menu removes the token from the Windows credential store.
- When uninstalling, tick "Delete instances, worlds and launcher settings".
- Data held by Microsoft and Mojang is deleted through their own services.

## Children

The launcher collects no data about anyone, children included. Age rules for Microsoft accounts are set by Microsoft.

## Changes and contact

New versions of this policy are published in the project repository: https://github.com/LunixSudo/selene-launcher. Questions can be asked there, under Issues.

Selene Launcher is not an official Minecraft product and is not approved by or associated with Mojang or Microsoft.
