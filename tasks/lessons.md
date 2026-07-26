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

---

## 5. Do not write a number into a doc comment before measuring it.

**What happened (2026-07-25):** Wrote module documentation for `variance.rs`
and `margin_model.rs` with the headline figures filled in from estimation
rather than from a run. Eleven of them were wrong across the session:

- `variance.rs` — SD at +400 was 2.04, not the 2.00 I wrote (that is the
  break-even figure, not the one at the required win rate); detection horizons
  were 1,440 / 6,640, not 1,500 / 6,400; a fixed cents move does *not* invert
  the CLV ranking, which needs unequal moves; a 3% edge at +600 loses 45% of
  seasons, not "far more often than not".
- `margin_model.rs` — implied correlation was −0.27 and +0.31, not −0.13 and
  +0.16; key-number margins land at 12.7% and 8.4%, not 9.4% and 5.8%; the
  reweighting pulls the mean margin from 3.50 to 3.20, which I had not
  predicted at all. And one claim was simply inverted: I asserted the cover
  curve *falls* across a range running from laying 14 points to getting 14,
  when covering plainly gets easier as the line moves.
- `docs.ts` — three of five "what changed since this was written" notes
  described articles I had not read. Two of those articles turned out to be
  *more* correct than the code they were written for.

**Why it matters:** every one of these was caught, because the figure went into
an assertion at the same time it went into the prose. That is the only reason
the cost was a rerun instead of a wrong number shipping in explanatory text —
which is exactly the failure mode the Phase 8 review already found four times,
where an InfoSection stated a figure the app contradicted on screen. Prose
reads as more authoritative than a number on a screen, so a wrong sentence
beats a right calculation.

**How to apply:** the claim and the test are one artifact. Write
`assert_relative_eq!(x, PLACEHOLDER)`, run it, read the actual value out of the
failure, and put *that* number in both places. Never the other order. And when
the subject is someone else's document, read the document — a correction notice
that mischaracterises the thing it corrects is worse than no notice.

---

## 6. When two independent methods agree exactly, suspect an identity.

**What happened (2026-07-25):** Noticed that `devig` returned bit-identical
results for Equal Margin and Shin on every two-way market. The instinct was
that this was the Phase 1 bug returning — a solver collapsing onto its
neighbour. It was not: for two outcomes, Shin's fair probabilities *are* the
equal-margin ones, provably, for whatever insider fraction balances the book.

**Why it matters:** the wrong conclusion in either direction is expensive. Had
I "fixed" it, I would have broken a correct implementation. Had I ignored it,
the app would go on presenting five methods as five opinions when on the most
common market shape it has four — and a user reading two agreeing columns as
corroboration is being misled by the UI, not by the maths.

**How to apply:** agreement to 1e-12 is not agreement, it is the same function.
Derive it before touching it. Then say so in the product: an identity the user
cannot see is a claim the interface is making on your behalf.

---

## 7. Build in public does not mean prepare a release.

**What happened (2026-07-25):** Interpreted "ready to start sharing" as a
request for tag-driven binary releases, signing guidance, and a release
process. The actual goal was day-one visibility: make the README easy to follow
while the project is still far from release.

**Why it matters:** Public source, usable development instructions, and a
published product are separate milestones. Adding release machinery changes
the project's posture and creates expectations the owner did not choose.

**How to apply:** When the user says they are building in public, default to
source-first documentation, clear work-in-progress language, and reproducible
local setup. Do not introduce tags, release automation, installers, or
download claims unless they explicitly ask to publish builds.
