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
- Worlds, servers and screenshots tabs; renaming worlds; joining a server straight from the
  launcher.
- Game console, crash analysis, play time and a button that stops a hung game.
- Search, sorting and groups on the home screen; copying an instance.
- Desktop shortcuts that start an instance.
- Skins page with a 3D preview, a skin library and cape selection; the player's head in the
  title bar. The character can idle, walk, run, fly with elytra or wave, and the capes sit in
  one row under it: the mouse wheel scrolls them and hovering one tries it on.
- Saving the skin the account wears now into the skin library.
- Effects in the settings, all off by default: seven live backgrounds drawn on the GPU (glowing
  rings that run through the colours, prism light rays with drifting dust, northern lights,
  liquid glass, out-of-focus lights, silky waves and a starry sky with shooting stars that fly
  in from the edge at different speeds, sometimes several at once), smooth page, card and dialog animations with a
  glowing Play button, and a soft click sound like the iOS keyboard. The settings preview each
  background as it really looks, and each instance can use the launcher's background, its own
  animated cover or any of the live ones.
- On the first start the launcher asks whether to favour speed or looks.
- Style presets in the settings: "Faster" and "Prettier" are built in, and the current theme,
  colour, font, background and effects can be saved under a name of your own.
- A setting for the faded edges of scrolling lists, which now also cover project pages,
  dropdowns and the version dialogs.
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
- Single player without an internet connection for an account that has signed in before, as
  in the official launcher.
- An instance page notice when the loader has a newer recommended build or mods, resource
  packs or shaders have updates, with a checkbox for each and one button that updates them.
  A refresh button in the instance header checks again.
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
- When the official hosts and their mirrors all fail, downloads and version lists come from
  the public mirror BMCLAPI. Files are still checked against the checksums of the
  official index, and a host that failed is skipped for five minutes so a blocked one does not
  cost a timeout per file. A loader file whose checksum cannot be had from anywhere is taken
  unchecked instead of failing, and checked on a later start. The privacy policy lists BMCLAPI
  and its download nodes.
- The mod catalog and mod downloads fall back to MCIM, a public copy of Modrinth and
  CurseForge, when those sites cannot be reached. Mod files keep the catalog checksums, and the
  CurseForge key is never sent to the copy.
- When Mojang's Java cannot be downloaded, the launcher installs Eclipse Temurin of the same
  version from Adoptium (or its TUNA mirror), then Azul Zulu, each checked against its
  vendor's SHA-256, and only then tries Mojang's Java on BMCLAPI.
- A temporary "Download source" setting that takes everything from BMCLAPI and MCIM, to check
  that the copies work. It goes away before the release.
- Importing a modpack, installing from the catalog and updating add-ons show their progress
  on the instance card and the instance page, file by file. These downloads go ahead of a game
  download running in the background, which waits for them.
- Stop asks the game to close as its window would, so it saves the world, and only kills it
  when it has not quit after 20 seconds.
- After a crash the launcher opens the instance, and the crash notice names the mod at fault
  and any enabled mods made for another game version.
- Deleting an instance or changing its game version while its game, Java or loader is
  downloading cancels the download. Mods, resource packs and shaders can be updated meanwhile.
- Changing the game version can also update resource packs and shaders for the new version
  and starts downloading it right away.
- The game starts in the system language the first time, unless the instance or modpack
  already has its own settings.
- A pixel font in the style of Minecraft (Pixelify Sans).
- Machine translations of the interface into Ukrainian, German, Spanish, Brazilian Portuguese,
  Polish, Turkish and Simplified Chinese. The language follows the system on first start.
- The Back link in the settings and on the skins page returns to the page you came from.

### Changed

- Starting an installed game no longer hashes every asset and library again: files that are
  in place with the right size are trusted, since they were checked when downloaded.
- The version list, version files, loader profiles and the Java list are kept on disk, so a
  slow or unavailable Mojang server does not hold up starting an installed game.
- Loader profiles are saved with their library hashes, which spares a dozen requests per launch.
- The add-ons list hashes files in parallel and remembers hashes of files that did not change.
- The game console receives output in batches instead of one message per line.
- Mods that CurseForge does not allow to download are looked up in one request.
- The installer puts the program into `%LOCALAPPDATA%\SeleneLauncher`, without a space.
- The privacy policy mentions that catalog pages load images from the sites their authors
  chose.
- New logo: a moon-rock cube with a golden crescent and no stars, with a simpler drawing for
  16–32 px icons. Sources are in `docs/brand`.
- The solid style is the default, so the launcher stays light on slower PCs.
- The solid style turns blur off completely, and the launcher follows the Windows setting that
  turns animations off.
- The game console stays light with thousands of lines.
- New instances start the game in fullscreen.
- "Prettier" now picks Mica and the northern lights.
- Worlds are no longer started from the launcher, which was unreliable across game versions;
  joining a server straight from the launcher stays.
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
- Downloads are named like `SeleneLauncher-Windows-Setup-0.1.0.exe` and
  `SeleneLauncher-Windows-Portable-0.1.0.zip`; the portable zip also carries the license.

### Fixed

- The worlds, servers and screenshots tabs no longer reread their folders on every download
  progress update.
- The update notice checks again only when the game version or loader changes, not after every
  settings change.
- An instance folder copied by hand shows up as its own instance, and two instances created at
  once never get the same folder.
- Loading more catalog results no longer breaks when a project shifts between pages.
- CurseForge modpacks put resource packs and shaders in their own folders and honour a custom
  overrides folder.
- Two installs downloading the same file at once no longer write into each other, and failed
  downloads leave no partial files.
- A Forge or NeoForge installer that could not be checked is fetched again instead of reused.
- Clicking a shortcut of a game that is already running keeps its Stop button.
- The console no longer shows lines twice when opened while the game prints.
- Closing the window during a plain download quits the launcher instead of leaving it hidden.
- Modrinth packs with extra dependency keys import the same way every time.
- A dependency pinned to a version for another game version or loader is replaced with one that
  fits.
- A loader installed while its checksums were unreachable starts offline too.
- When Java comes from Temurin or Zulu, later launches reuse it without asking the network.
- CurseForge file lists are sorted newest first, also when they come from the mirror.
- CurseForge fingerprints are kept, so content lists no longer reread every mod.

- The current skin and capes show on the skins page: Mojang's texture links are loaded over
  HTTPS.
- Opening the launcher again while a game runs that it started shows the Stop button.
- The skins page no longer asks Mojang for the profile over and over, which made saving a
  skin or changing a cape fail. Busy answers from Minecraft Services are tried again, and
  "too many requests" gets its own message.
- The 3D model turns smoothly and fits small screens, so the capes under it stay in view.
- Live backgrounds no longer show colour streaks over Mica and the solid style, and hovering an
  instance no longer draws a square around it on some graphics cards. Over the solid style the
  rings are drawn opaque and in full precision, which removes darker rectangles around text on
  older GPUs, and the seam on their left side is gone.
- Forge build lists no longer fail when Forge's promotions file cannot be loaded.
- Loader versions, NeoForge's especially, load more reliably: a failed request is tried
  again, then the list saved last time is shown.
- Hovering buttons in the solid style is visible in the dark theme.
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
- Sign-in ignores connections to its local port that never send a request, such as a browser
  preconnecting, and answers a callback with a foreign state with an error page.
- Modpack import only downloads over HTTPS and from the hosts Modrinth allows, trying the next
  listed link when the first one is not allowed.
- Starting a second game of the same version no longer fails on natives the first one holds open.
- Server lists and the skin library are written atomically, like accounts and settings.
- Instance memory set by hand stays within what the PC has.
- Links in project descriptions and in the privacy policy open in the browser instead of inside
  the launcher, also with a middle or Ctrl click.
- Dialogs are announced by their title to screen readers.
- A slow catalog search can no longer replace the results of a newer one.
- A mod loader that could not be reached shows its error under the loader and no longer
  keeps it on screen after picking another loader or game version.
- Downloads wait a moment before trying again, skip retries for files that do not exist and
  clean up what a failed one left behind.
- Files that an antivirus holds for a moment after they are written no longer fail the
  download or the save.
- A full disk and a PC without enough free memory for the game get their own messages.
- The portable version says it needs the WebView2 runtime instead of closing silently.
- The skins page says when the PC has no WebGL for the 3D preview.
- The home screen and the settings show when instances could not be read or settings
  could not be saved.
- Updating only the loader from the instance page notice keeps the game version, and a fresh
  install no longer triggers a second update check.
- Modpack files in `client-overrides` win over the ones in `overrides`, and CurseForge packs
  use the overrides folder their manifest names.
- A saved catalog or version list is used when the server answers with broken JSON.
- The launcher stops waiting for the game's output a few seconds after the game exits.
- "Update all" downloads every new file first and only then swaps them, so a failed download
  leaves the old mods in place.
- Two Forge or NeoForge installers no longer run at the same time for one build.
- Deleting an instance while its modpack is importing cancels the import.
- Installing from the catalog reuses the dependency list from the preview instead of asking
  the servers again, and dependencies are looked up one tree level at a time in parallel.
- Modrinth and CurseForge are asked about each installed file and the Java runtime index once
  per session.
- NeoForge builds are listed from BMCLAPI when the NeoForge site cannot be reached.
- Changing the loader build or the game version while the game downloads goes on with the
  download for the new one instead of stopping it.
- A first start of versions before 1.11 sets the system language too.
- Uninstalling removes the instance shortcuts from the desktop.
- The lifted instance card's shadow is no longer cut off at the edges of the list.
- Dialogs no longer flash while they open.
