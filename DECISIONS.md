# DECISIONS.md — decision ledger

A lightweight, cathedral-inspired record of the project's load-bearing decisions.
This is **not** full cathedral compliance (no `Premise: cathedral`, no
`blueprints/`); it's the pragmatic middle ground: each decision preserves the
alternatives considered, who approved it, and a **falsification condition** — the
concrete evidence that should make us revisit it. `DESIGN.md` holds the current
design; this file holds *why* and *what we'd need to change our minds*.

Template per entry: **Context · Alternatives considered · Decision · Rationale ·
Decided by · Falsification condition**. Status is `ACCEPTED` until superseded.

---

## DL-001 — Pin Bevy to exactly `=0.18.1` · ACCEPTED (Phase 0)

- **Context:** Bevy's API changes meaningfully between releases; the project
  needs reproducible builds and deliberate upgrades.
- **Alternatives considered:** caret range `0.18` (allow patch drift); track
  latest; use a release candidate (`0.19.0-rc`).
- **Decision:** exact pin `bevy = "=0.18.1"`; `Cargo.lock` committed; CI builds
  with `--locked`.
- **Rationale:** latest *stable* at project start (verified on crates.io);
  exact pin makes any version change an explicit, reviewed edit.
- **Decided by:** author, matching the project constraint in `CLAUDE.md`.
- **Falsification condition:** a needed feature/bugfix exists only in a newer
  Bevy, or `0.18.1` has a blocking defect → then bump as a deliberate migration
  task (expect API churn), not casually.

## DL-002 — Rust edition 2024 on the `stable` toolchain · ACCEPTED (Phase 0)

- **Context:** need a toolchain baseline for everyone + CI.
- **Alternatives considered:** edition 2021; pin an exact rustc version.
- **Decision:** edition 2024; `rust-toolchain.toml` channel = `stable`.
- **Rationale:** latest stable language; Bevy 0.18 MSRV (1.89) is well below the
  installed toolchain; `stable` avoids forcing re-downloads in CI.
- **Decided by:** author.
- **Falsification condition:** a dependency requires a newer MSRV than `stable`
  provides, or we need a nightly-only feature → reconsider pinning.

## DL-003 — One Bevy `Plugin` per game system · ACCEPTED (Phase 0)

- **Context:** keep the codebase decoupled and `main` readable as it grows.
- **Alternatives considered:** a few large modules; a single mega-plugin;
  feature-folders without the plugin boundary.
- **Decision:** each system is its own `Plugin` in `src/plugins/<system>.rs`;
  `main` is just the plugin list.
- **Rationale:** idiomatic Bevy; clean ownership (components = data, systems =
  behavior, resources = global state); easy to add/remove systems.
- **Decided by:** project requirement (kickoff brief) + author.
- **Falsification condition:** plugin boundaries cause excessive cross-plugin
  coupling or ordering pain that a different decomposition would remove.

## DL-004 — Flight model = damped arcade · ACCEPTED (Phase 1)

- **Context:** Phase 1 needs a flight feel; this is a "big fork" with real
  tradeoffs.
- **Alternatives considered:** Newtonian/inertial (flight-assist off); a hybrid
  with an assist toggle.
- **Decision:** damped arcade — throttle sets a target speed; angular velocity
  ramps to input and damps to zero on release (auto-stabilize).
- **Rationale:** closest to the 1984 original's feel, easiest to fly and tune;
  Newtonian can be added later behind a toggle.
- **Decided by:** **user-approved** (explicit options prompt: damped-arcade vs.
  Newtonian vs. hybrid).
- **Known shortcut:** per-entity state is on the `Ship` component, but tuning is
  a single global `FlightConfig` resource and the integrator is gated to
  `Single<.. With<Player>>`. A second ship hull or AI ship means a Resource →
  Component move, a `Single` → `Query` rewrite, and a separate movement system
  for non-player craft. Acceptable for Phase 1; revisit when combat (Phase 2)
  introduces other ships.
- **Falsification condition:** hands-on playtesting shows the feel is wrong for
  the game's identity (e.g., players want inertial drift) → add a
  flight-assist-off / Newtonian mode.

## DL-005 — Camera = first-person cockpit · ACCEPTED (Phase 1)

- **Context:** how the player views flight; affects legibility and immersion.
- **Alternatives considered:** third-person chase camera.
- **Decision:** first-person cockpit `Camera3d`, mounted as a child of the ship
  (transform propagation handles the follow).
- **Rationale:** immersive and Elite-authentic; child-of-ship keeps the
  implementation trivial and correct.
- **Decided by:** **user-approved** (explicit options prompt: cockpit vs. chase).
- **Falsification condition:** usability testing shows first-person is too
  disorienting without more cockpit framing/reference → add a chase mode or a
  cockpit overlay, or make it toggleable.

## DL-006 — Starfield = Fibonacci shell, follows player translation only · ACCEPTED (Phase 1)

- **Context:** need a star backdrop that conveys rotation but not translation
  parallax (distant-star feel).
- **Alternatives considered:** skybox cubemap (needs a texture asset); a single
  custom point-cloud mesh; stars fixed in world space (would let the ship fly
  past/through them).
- **Decision:** ~1400 stars on a deterministic Fibonacci sphere, shared
  mesh+unlit material, parented to a root that copies the player's translation
  (not rotation) each frame.
- **Rationale:** deterministic, no RNG crate, batch-friendly, asset-free; the
  translation-only follow gives equidistant stars that still show rotation.
- **Decided by:** author (implementation-level; not raised as a fork).
- **Falsification condition:** profiling shows the entity-per-star approach is
  too costly at the target scale, or we want richer skies → switch to a skybox
  cubemap or an instanced/custom-mesh starfield.

## DL-007 — Lighting = two directional lights (no `AmbientLight`) · ACCEPTED (Phase 1)

- **Context:** reference objects need shading; their dark sides shouldn't be
  pure black.
- **Alternatives considered:** a key `DirectionalLight` + `AmbientLight`/
  `GlobalAmbientLight` (the 0.18 ambient API was ambiguous to wire);
  image-based / environment-map lighting.
- **Decision:** a key `DirectionalLight` plus a dim fill `DirectionalLight` from
  the opposite side.
- **Rationale:** simple, cheap, uses only API already verified to compile; gives
  fill on shadowed faces without an ambient term.
- **Decided by:** author (implementation-level).
- **Falsification condition:** we need true ambient/IBL for look or correctness →
  add `AmbientLight`/environment maps once that API is verified.

## DL-008 — Never commit/push without explicit approval · ACCEPTED (process)

- **Context:** the kickoff brief originally authorized "commit in logical
  increments"; the user later revoked standing commit authorization.
- **Alternatives considered:** keep auto-committing verified increments.
- **Decision:** prepare and verify changes, keep them in small logical
  increments, but **never `git commit`/`git push` without the user's explicit
  go-ahead** (recorded in `CLAUDE.md`).
- **Rationale:** the user oversees what lands in history.
- **Decided by:** **user-directed.**
- **Type:** standing process directive — *not falsifiable by evidence*. Unlike
  the design decisions above, no accumulating data revisits it; it stands until
  rescinded.
- **Revisit trigger:** the user explicitly rescinds it, or sanctions a scoped
  carve-out (e.g. an automated CI/release flow permitted to commit within set
  bounds).

## DL-009 — Phase-1 reference geometry is a tracked placeholder · ACCEPTED (Phase 1)

- **Context:** the flight slice needs visible objects to make motion legible
  before real content systems exist.
- **Alternatives considered:** an empty void (motion illegible); investing in
  real station/asteroid art now (premature — those are Phase 5 / later).
- **Decision:** ship a ring station (torus + hub) and a few asteroids in
  `world.rs` as explicit stand-ins, clearly labeled as placeholders.
- **Rationale:** cheap, legible reference frame now; no wasted art before the
  owning systems exist.
- **Decided by:** author (implementation-level).
- **Retirement condition:** replace when real stations land (Phase 5) and
  asteroids gain a mining/field system (later). Until then, the "placeholder /
  stand-in" labeling in `world.rs` must stay honest — undocumented placeholder
  content shipping as if final would be the violation.

---

## How decisions get added

When a "big fork" (per `CLAUDE.md`) is resolved, add an entry here with its
alternatives and falsification condition. Mark an entry `SUPERSEDED by DL-NNN`
rather than deleting it, so the reasoning trail is preserved.

**Recording rejections (ledger symmetry).** Rejected options are normally kept
inline in each entry's *Alternatives considered* — the lightweight symmetry
mechanism for this project. A fork that is *considered and turned down without an
accompanying approval* (e.g. a future "should we adopt full cathedral blueprints?"
that lands as "no") gets its own `DL-NNN … REJECTED` entry, so the "no" is
remembered and not silently re-litigated later.
