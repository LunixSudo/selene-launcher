# Contributing

## Before you push

```sh
pnpm lint
pnpm check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same commands.

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
- `unsafe` is forbidden across the workspace.

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
change: the license page shows the privacy policy in the installer's language. The texts live
in `src-tauri/nsis/privacy/` and must say the same as `PRIVACY.md` and `PRIVACY.ru.md`. When
upgrading the Tauri CLI, take the new template from `crates/tauri-bundler` and apply the
change again; do the same for `English.nsh` and `Russian.nsh`.

## Docs

Update `README.md` and the `Unreleased` section of `CHANGELOG.md` in the same commit as the
change. If the launcher starts talking to a new service or storing something new, update
both privacy policies and the installer texts too.

## Comments

Explain why, not what. If a comment restates the code, delete it.
