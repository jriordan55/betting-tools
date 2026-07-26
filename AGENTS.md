# Repository Guidelines

## Project Structure & Module Organization

This is a Tauri v2 desktop app with a SvelteKit frontend and Rust workspace.

- `crates/bettor-core/`: pure betting math; keep it independent of Tauri, I/O, and global state.
- `src-tauri/`: thin desktop shell, IPC commands, window setup, and SQLite bet-log storage.
- `src/`: Svelte 5 UI. Routes live in `src/routes/`; shared calculators, charts, and UI components live in `src/lib/`.
- `static/` and `src-tauri/icons/`: web and application assets.
- `docs/`: testing strategy and documented behavioral divergences.

Do not hand-edit `src/lib/bindings.ts`; Rust tests regenerate it from command signatures.

## Build, Test, and Development Commands

- `pnpm install`: install frontend and Tauri tooling.
- `pnpm tauri dev`: run the desktop application locally.
- `pnpm check`: run Svelte and TypeScript diagnostics.
- `pnpm test`: run Vitest once; use `pnpm test:watch` during development.
- `pnpm rs:test`: run all Rust workspace tests.
- `pnpm rs:fmt`: format Rust sources.
- `pnpm verify`: required pre-review gate—Clippy, Rust tests, Svelte checks, Vitest, and production build.
- `pnpm tauri build`: create a distributable desktop bundle.

## Coding Style & Naming Conventions

Use TypeScript strict mode, Svelte 5 runes, and existing CSS variables; do not introduce Tailwind. Follow local formatting: tabs in Svelte/TypeScript and standard `rustfmt` output in Rust. Name Svelte components in `PascalCase`, TypeScript helpers in `camelCase`, and Rust modules/functions in `snake_case`. Keep calculations in `bettor-core`; frontend code should format, visualize, and invoke typed commands.

## Testing Guidelines

Place frontend tests beside source as `*.test.ts`. Keep Rust unit tests near their modules and integration tests under `crates/bettor-core/tests/`. Every bug fix or deliberate behavior change needs a focused, descriptively named test. Golden fixtures in `tests/fixtures/` are committed reference data; never regenerate them merely to make a failure pass. Run `pnpm verify` before declaring work complete.

## Commit & Pull Request Guidelines

Follow the existing Conventional Commit style: `feat(scope): summary`, `fix: summary`, `test(ui): summary`. Keep subjects concise, lowercase, and imperative. Before committing, review and refactor the diff, then rerun verification. Pull requests should explain the problem and solution, link relevant issues, list validation performed, and include screenshots for visible UI changes. Call out database migrations, generated-binding changes, or intentional parity divergences explicitly.
