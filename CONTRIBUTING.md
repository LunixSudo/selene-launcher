# Contributing

## Before you push

```sh
pnpm lint
pnpm check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands on Windows and Linux, and three more checks:

- `cargo deny check` (rules in `deny.toml`): known vulnerabilities, licenses that cannot be
  combined with the GPL, and crates from anywhere but crates.io. Run it after touching
  `Cargo.toml`; `cargo install cargo-deny --locked` installs it.
- [zizmor](https://docs.zizmor.sh/) on the workflows in `.github`.
- A dependency review on pull requests, which fails when a change brings in a library with
  a known vulnerability.

The Rust version is pinned in `rust-toolchain.toml` and the Node.js version in
`.node-version`, so CI and your machine use the same ones; rustup installs the pinned
toolchain by itself.

Dependabot opens weekly pull requests for Cargo, npm, the Rust toolchain and GitHub Actions,
a week after a version comes out. Minor and patch updates come grouped; check the changelog
of a major one before merging it.

## Releases

Pushing a `v*` tag runs the Build workflow without caches and leaves a draft release with
the installers, `SHA256SUMS.txt`, a list of dependencies (SPDX) and a signed build
provenance statement. The notes are taken from the version's section of `CHANGELOG.md`, and
the tag must match the version in `package.json`. The draft is published by hand.

Labels are kept in `.github/labels.json`; the Labels workflow creates and updates them. The
branch and tag rules are kept in `.github/rulesets/` and imported in the repository settings.

## Branches

Work happens on `dev`, and pull requests target it. `main` holds released code only: each
release lands there as a single commit tagged with its version.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`,
`perf:`, `docs:`, `test:`, `chore:`, `ci:`. Add a scope when it helps, e.g.
`feat(auth): refresh expired tokens`. Keep commits small and focused on one change.

## Rust

- `snake_case` for functions, variables and modules; `PascalCase` for types and traits;
  `SCREAMING_SNAKE_CASE` for constants and statics.
- File order: `use` statements (std, then external crates, then `crate::`), constants, types
  and their `impl` blocks, free functions, then `#[cfg(test)] mod tests` at the bottom.
- Return errors with `?` instead of panicking. `unwrap` and `expect` are fine in tests and for
  invariants that truly cannot fail.
- `unsafe` is denied across the workspace. The Win32 modules in `src-tauri/src`
  (`backdrop.rs`, `game_window.rs`, `gpu.rs` and `hardware.rs`) allow it, and every block
  there has a `SAFETY` comment.

## TypeScript and Svelte

- `camelCase` for variables and functions, `PascalCase` for types and components,
  `UPPER_SNAKE_CASE` for module-level constants.
- Components live in `src/lib/components/` and are named `PascalCase.svelte`. Other modules use
  `kebab-case.ts`.
- Component order: `<script lang="ts">` (imports, `$props`, `$state`, `$derived`, functions),
  then markup, then `<style>`.
- Use the CSS variables from `src/lib/styles/global.css` for colors, radii and timing.

## Installer

`src-tauri/nsis/installer.nsi` is Tauri's NSIS template from `@tauri-apps/cli` 2.12.4 with one
change: the license page shows the privacy policy in the installer's language. The texts in
`src-tauri/nsis/privacy/` are written from `PRIVACY.md` and `PRIVACY.ru.md` by
`scripts/privacy.mjs` on every `pnpm build`, so they are never edited by hand. When
upgrading the Tauri CLI, take the new template from `crates/tauri-bundler` and apply the
change again; do the same for `English.nsh` and `Russian.nsh`.

When the privacy policy changes, bump `PRIVACY_VERSION` in
`src-tauri/src/commands/privacy.rs` and in `src-tauri/nsis/hooks.nsh` (a test checks they
match). Users then accept the new version once after updating.

## Docs

Update `README.md` and the `Unreleased` section of `CHANGELOG.md` in the same commit as the
change. If the launcher starts talking to a new service or storing something new, update
both privacy policies and the installer texts too.

## Comments

Explain why, not what. If a comment restates the code, delete it.
