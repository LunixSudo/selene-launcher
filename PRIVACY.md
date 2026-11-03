# Selene Launcher Privacy Policy

Effective November 3, 2026.

Selene Launcher is a free, open-source launcher for Minecraft: Java Edition. It has no servers of its own: its developers do not receive or store any data about you.

In short:

- The launcher never sees your Microsoft password.
- Everything it saves stays on your computer.
- It connects only to the services listed below, and only when a feature needs them.
- It collects no statistics or analytics, shows no ads, and does not sell or share data with anyone.

## What stays on your computer

- **Sign-in.** You sign in on a Microsoft page. The token used to sign you in again is not kept in a plain file but in the system credential store: Credential Manager on Windows, the Secret Service keyring on Linux. Your player name, UUID and skin are saved in the launcher folder to show your accounts.
- **Game data.** Instances, worlds, mods, saved skins, screenshots, crash reports, play time and settings live in `%APPDATA%\SeleneLauncher` (on Linux `~/.local/share/SeleneLauncher`), or next to the program in the portable version. For Java 19 and newer, the launcher also keeps a `classes.jsa` file next to each instance, so the game starts faster next time. It holds only the code of the game and its mods.
- **The launcher log.** `logs\launcher.log` records:
  - the launcher version and your system;
  - the models of your processor, graphics card, memory sticks and motherboard, never their serial numbers;
  - free disk space;
  - the Java arguments of each start and how long the game took to load;
  - which download sources answered and why some failed;
  - game starts, exits and errors;
  - the last lines the game printed before a crash.

  It holds no passwords, tokens or keys, and it is never sent anywhere by itself.
- **The OptiFine window.** Like any browser, it keeps the site's cookies and cache, in `%LOCALAPPDATA%\io.github.lunixsudo.selene`.
- **Settings file.** "Save to file" in the settings writes the look and launcher settings to a file you choose. It holds no accounts, tokens or instances.

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
| Modrinth and CurseForge | The catalog, installing and updating mods and modpacks | Search queries, and checksums of installed files to recognise them and find updates |
| MCIM (mod.mcimirror.top) | A spare copy of the Modrinth and CurseForge catalogs and mod files, used only when those sites cannot be reached. Mod files are checked against the catalog checksums, which come from MCIM too while the catalog itself is down. The launcher's CurseForge key is never sent there | Search queries, checksums of installed files |
| OptiFine (optifine.net) | The official OptiFine site, opened in a launcher window only when you press "Install OptiFine". Before that, the mods tab of a Forge instance reads its downloads page, at most once every 10 minutes, to see whether there is a build for the game version that is not installed yet. The site and its ads work as in any browser | What any website sees: your IP address and what you click |
| GitHub | Checking for and downloading launcher updates. Can be turned off in settings | Nothing beyond the request itself |
| Discord (the app on your computer) | The status in your Discord profile. It can be turned off in settings, and then the launcher does not connect to Discord at all | The instance name, game and loader version and when the game started, or "In the launcher" |

Every request carries the launcher's name and version, as Modrinth asks of all apps. Like any website, these services see your IP address and handle data under their own policies. The launcher sends them nothing beyond what a feature needs.

Project pages in the catalog show icons, descriptions and galleries chosen by the authors of the projects. Their images can be hosted on other sites, which then also see your IP address when the page is opened.

## Reporting a problem

"Report a problem" in the settings opens a new bug report on GitHub in your browser. The report is filled in with:

- what you wrote;
- the launcher version, your system and the models of your hardware;
- if you choose, the end of the launcher log, with your Windows user name hidden.

The whole log, with the user name hidden too, is saved to `%TEMP%\SeleneLauncher\selene-report.log`, so you can attach it. Nothing is sent until you press the button on GitHub yourself. Reports on GitHub are public, so read the text before you send it.

## The game itself

Minecraft is a Microsoft product. While you play, the game connects to Mojang and Microsoft and sends them its own data, telemetry included, under the [Microsoft Privacy Statement](https://privacy.microsoft.com/privacystatement). Servers you join see your player name, skin and IP address, and mods work under the rules of their authors. The launcher has no say in any of this.

## Security

- Sign-in tokens are kept only in the system credential store and never get into files or logs.
- Downloaded files are checked against the checksums their sources publish, and a file that does not match is thrown away.
- The source code is open: anyone can check what the launcher does.
- Please report a vulnerability privately, as described in the [security policy](https://github.com/LunixSudo/selene-launcher/security/policy).

## Deleting your data

- "Sign out" in the account menu removes that account's token from the credential store.
- When uninstalling, tick "Delete instances, worlds and launcher settings". This also removes the sign-in tokens of all accounts and the data of the OptiFine window.
- In the portable version, sign out of your accounts and delete the program folder.
- Data held by Microsoft and Mojang is deleted through their own services.

## Children

The launcher collects no data about anyone, children included. Age rules for Microsoft accounts are set by Microsoft.

## Changes and contact

When this policy changes, the launcher shows the new version on its next start. New versions are published in the project repository: https://github.com/LunixSudo/selene-launcher. Questions can be asked there, under Issues.

Selene Launcher is not an official Minecraft product and is not approved by or associated with Mojang or Microsoft.
