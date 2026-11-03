# Selene Launcher Privacy Policy

Effective October 29, 2026.

Selene Launcher is a free, open-source launcher for Minecraft: Java Edition. It has no servers of its own: its developers do not receive or store any data about you.

## What stays on your computer

- **Sign-in.** You sign in on a Microsoft page, and the launcher never sees your password. The token used to sign you in again is kept in the Windows credential store (Credential Manager), or on Linux in the Secret Service keyring, not in a plain file. Your player name, UUID and skin are saved in the launcher folder to show your accounts.
- **Game data.** Instances, worlds, mods, saved skins, logs, crash reports and settings live in `%APPDATA%\SeleneLauncher`, or next to the program in the portable version. The launcher's own log, `logs\launcher.log`, records the launcher version, your system, the models of your processor, graphics card, memory sticks and motherboard (never their serial numbers), free disk space, the Java arguments of each start, which download sources answered and why some failed, game starts and exits, errors, and the last lines the game printed before a crash; it holds no passwords, tokens or keys. It leaves your PC only if you attach it to a bug report yourself; "Report a problem" in the settings hides your Windows user name in it first.

None of this is sent anywhere, except as described below.

## What the launcher connects to

| Service | Why | What is sent |
| --- | --- | --- |
| Microsoft and Xbox Live | Signing in | Sign-in tokens |
| Mojang and Minecraft Services | Checking you own the game, downloading the game and Java, skins and capes, creating and changing your player name | Your Minecraft token, the skin you choose, the player name you type |
| Fabric, Quilt, Forge and NeoForge | Mod loaders: the list of builds and the loader itself, which Forge and NeoForge download with their own installer. When a site cannot be reached, the same files come from the official mirrors the project lists, for Forge that is CreeperHost. Libraries of other projects that the loaders use, like ASM, are taken from Maven Central (repo1.maven.org) first | The game version |
| BMCLAPI (bmclapi2.bangbang93.com, run by bangbang93) | A spare copy of the game, Java and loader files, used only when the official servers and their mirrors cannot be reached. BMCLAPI hands each file out from one of its download nodes, run by volunteers and Chinese university mirrors (CERNET), which also see your IP address. Files are checked against the official checksums whenever those can be had | The address of the requested file |
| Adoptium (api.adoptium.net, run by the Eclipse Foundation), GitHub and the TUNA mirror (mirrors.tuna.tsinghua.edu.cn, Tsinghua University) | Eclipse Temurin Java, used only when Mojang's Java cannot be downloaded. The file is checked against Adoptium's SHA-256 checksum | The Java version, your system and processor type |
| Azul (api.azul.com, cdn.azul.com, Azul Systems) | Azul Zulu Java, used only when neither Mojang's Java nor Temurin can be downloaded. The file is checked against Azul's SHA-256 checksum | The Java version, your system and processor type |
| MCIM (mod.mcimirror.top) | A spare copy of the Modrinth and CurseForge catalogs and mod files, used only when those sites cannot be reached. Mod files are checked against the catalog checksums, which come from MCIM too while the catalog itself is down. The launcher's CurseForge key is never sent there | Search queries, checksums of installed files |
| Modrinth and CurseForge | The catalog, installing and updating mods | Search queries, and checksums of installed files to recognise them and find updates |
| OptiFine (optifine.net) | The official OptiFine site, opened in a launcher window only when you press "Install OptiFine". Before that, the mods tab of a Forge instance reads its downloads page, at most once every 10 minutes, to see whether there is a build for the game version that is not installed yet. The site and its ads work as in any browser | What any website sees: your IP address and what you click |
| GitHub | Checking for launcher updates | Nothing beyond the request itself |
| Discord (the app on your computer) | The "Playing Selene Launcher" status | The game version and instance name. Can be turned off in settings |

Like any website, these services see your IP address and handle data under their own policies. The launcher sends them nothing beyond what a feature needs.

Project pages in the catalog show icons, descriptions and galleries chosen by the authors of the projects. Their images can be hosted on other sites, which then also see your IP address when the page is opened.

The launcher does not collect statistics or analytics, show ads, or sell or share data with anyone.

## Deleting your data

- "Sign out" in the account menu removes the token from the credential store.
- When uninstalling, tick "Delete instances, worlds and launcher settings".
- Data held by Microsoft and Mojang is deleted through their own services.

## Children

The launcher collects no data about anyone, children included. Age rules for Microsoft accounts are set by Microsoft.

## Changes and contact

New versions of this policy are published in the project repository: https://github.com/LunixSudo/selene-launcher. Questions can be asked there, under Issues.

Selene Launcher is not an official Minecraft product and is not approved by or associated with Mojang or Microsoft.
