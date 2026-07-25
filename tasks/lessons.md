# Lessons

Patterns from corrections, written so the same mistake is not repeated. Read
this at the start of a session alongside `CLAUDE.md`.

---

## 1. The plan is the plan. Execute it.

**What happened (2026-07-25):** Asked which calculator should fill the Phase 4
vertical slice when `tasks/todo.md` already contained the plan, the
recommendation, and 21 finished calculators in the reference repo to port. The
answer cost an evening of back-and-forth and produced nothing.

**Why it matters:** A question is only worth asking when proceeding under any
reasonable assumption would be unsafe or would waste the work if wrong. "Which
of two orderings should I use" is not that — it is a judgment call, and making
it is the job.

**How to apply:** If `tasks/todo.md` names the work, start the work. State the
assumption in one line and keep going. Save questions for the end, attached to
finished output, where they cost nothing.

---

## 2. A passing type-check says nothing about whether a UI works.

**What happened:** Reported "the app runs" on the strength of `cargo test`,
`clippy`, `svelte-check` and a clean `vite build`. Two layout bugs followed
immediately — a mobile breakpoint that stacked the sidebar over the content at
the default window size, and a `display: none` sidebar paired with a `0` grid
column, which put every calculator inside the zero-width track. Both render as
valid CSS. Neither tool can see them.

**Why it matters:** "Never mark a task complete without proving it works" means
proving the *thing the user asked for* works. For a desktop UI, the build
passing is a precondition, not the proof.

**How to apply:** Say precisely what was verified and what was not — "clippy,
tests and build pass; I have not seen the window render" is honest, "the app
runs" is not. Adam is at the machine and can see it; ask him to look rather than
burning turns trying to screenshot his screen. Do not narrate a UI as working
from a green build.

**Fixed as a result:** `pnpm verify` now runs `pnpm build`. The prerender crash
on `/calculators/[slug]` sailed straight past the old gate.

---

## 3. Deleted code is a decision until proven otherwise.

**What happened:** Found `ParlayCorrelation.tsx` in the reference repo — 505
lines, in no registry, no route, imported by nothing — assumed the omission was
an oversight, and shipped it as a 22nd calculator. Adam had cut it deliberately:
the correlation parameter is unidentifiable, for three separate structural
reasons now written up in `tasks/todo.md` under "Deliberately not built".

**Why it matters:** Scope was "port the calculators that exist". Something built
and then removed usually means someone tried it and learned something. Restoring
it without asking discards that knowledge and re-ships a known dead end.

**How to apply:** Orphaned code is out of scope by default. If it looks worth
resurrecting, ask why it was cut *before* building on it — one sentence, at the
point of discovery. And when cutting something for a real reason, write the
reason down where the next person will read it, which is what "Deliberately not
built" now exists for.

---

## 4. Do not widen scope on your own judgment.

**Running theme across all three.** The extra calculator, the questions before
starting — each was defensible in isolation and each moved away from what was
asked. Deliver the requested scope in full, note anything adjacent worth doing,
and let Adam decide whether to take it on.
