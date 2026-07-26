# Contributing

Thanks for helping improve Bettor Desktop. The project is under active
development, so discuss substantial behavior or architecture changes in an
issue before investing in an implementation.

## Development workflow

1. Read [AGENTS.md](AGENTS.md) for repository structure, conventions, and the
   required review workflow.
2. Install dependencies with `pnpm install`.
3. Create a focused branch and make the smallest complete change.
4. Run `pnpm verify` before requesting review.

Keep betting calculations and domain rules in `crates/bettor-core/`. The
`src-tauri/` crate should remain a thin desktop, persistence, and IPC layer;
`src/` owns presentation, formatting, and typed command invocation.

Do not hand-edit `src/lib/bindings.ts`; Rust command signatures generate it.
Golden files in `tests/fixtures/` are reference behavior and must not be
regenerated merely to silence a failure. Explain any intentional fixture or
parity change in the pull request.

## Changes and review

Use Conventional Commit subjects such as `fix(ui): correct implied odds`.
Include focused tests for bug fixes and behavior changes. Pull requests should
describe the problem and solution, link related issues, list verification
performed, and include screenshots for visible changes. Call out database
migrations, generated-binding updates, and intentional behavior divergences.
