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
- **Axis update (see DL-011):** the *number of rotational axes* (whether yaw
  exists) is revisited in DL-011 — faithful no-yaw becomes the default. The
  damped-arcade *integration model* recorded here stands unchanged.
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

## DL-010 — Canonical version baseline = BBC disc / enhanced superset · ACCEPTED (Phase 1.5, gameplay design)

- **Context:** "the 1984 game" is not one artifact — the cassette, BBC **disc**,
  the Acorn **Electron**, and the home-computer **ports** (C64, NES, Spectrum,
  Archimedes…) differ in the buy list, mining, missions, bestiary, and docking.
  A faithful remake must fix one canonical feature set, because the choice cuts
  across nearly every subsystem (see `GAMEPLAY.md` §2).
- **Alternatives considered:** **cassette-pure** (pulse+beam only, no missions, no
  mining, instant docking); **defer** the decision to Phase 3; **track a specific
  port** (e.g. C64, which adds Trumbles + a soundtrack).
- **Decision:** the **BBC disc / enhanced superset** is canonical — it has the
  missions (Constrictor, Thargoid Plans), asteroid mining + Rock Hermit, and the
  full laser line-up (incl. military/mining), but **excludes** port-only extras
  (Blue Danube docking music, Trumbles, soundtracks, port-only missions). A
  `cassette-pure` mode can be a later toggle.
- **Rationale:** the fullest expression of the game on its *native* hardware;
  maximises faithful content without importing port-era additions that aren't
  "the BBC original".
- **Decided by:** **user-approved** (explicit options prompt: disc-superset vs.
  cassette-pure vs. defer).
- **Falsification condition:** disc-only content proves to need port-era specs we
  cannot source from primary references, or the Owner wants strict cassette
  fidelity → fall back to the `cassette-pure` baseline.

## DL-011 — Flight axes = faithful no-yaw default + optional yaw-assist toggle · ACCEPTED (Phase 1.5)

- **Context:** the 1984 Cobra Mk III has only **pitch + roll** (no yaw); aiming is
  "roll the target into the pitch plane, then pitch onto it". Phase 1 shipped
  roll + pitch + **yaw** (DL-004, A/D = yaw). The project mandate is faithfulness
  to the original, so the extra axis conflicts with shipped code
  (see `GAMEPLAY.md` §8).
- **Alternatives considered:** **keep 3-axis yaw** (modern arcade feel, diverges
  from the original's defining handling); **strict no-yaw** with no toggle at all.
- **Decision:** **no-yaw (pitch + roll + throttle) is the default**; an optional
  **"yaw assist" mode behind a toggle** re-enables a yaw axis for players who want
  it. Default control map becomes **W/S = pitch, A/D = roll, R/F = throttle**, with
  **Q/E = yaw only while assist is on** (toggle key `Y`). The damped-arcade
  integration model (DL-004) is unchanged — only the available axes change.
- **Rationale:** preserves the original's defining flight feel by default while
  keeping a modern accessibility option; aligns combat aiming, enemy AI, and the
  docking roll-match with the faithful 2-axis model.
- **Decided by:** **user-approved** (explicit options prompt: faithful-drop-yaw
  vs. keep-3-axis vs. hybrid).
- **Falsification condition:** hands-on playtesting shows no-yaw-default is too
  hard with a keyboard / modern expectations and players keep the assist on by
  default → reconsider making yaw the default (or revisit the control mapping).

## DL-012 — Laser hit detection = analytic ray-vs-sphere (instant hit) · ACCEPTED (Phase 2)

- **Context:** the 1984 lasers are *instant-hit, no projectile travel* — the
  engine compares the squared crosshair distance to a per-ship targetable area
  (Cobra Mk III = `95×95 = 9025`); see `GAMEPLAY.md` §10.
- **Alternatives considered:** physical projectile/tracer entities (rejected —
  the original lasers have no travel time); screen-space crosshair pixel pick
  (needs camera projection, diverges from a muzzle-origin ray); a 3D dot-product
  cone test.
- **Decision:** cast `Ray3d::new(muzzle, transform.forward())`, wrap once in
  `RayCast3d`, and test `sphere_intersection_at` against each targetable's
  `BoundingSphere` (built from `Transform` + `CollisionRadius`); the nearest
  positive hit takes damage. No projectile entity for lasers.
- **Rationale:** closed-form, exact, allocation-free; faithful analog of the
  squared-radius test; uses only `bevy_math` (no physics crate, per DL-014).
- **Decided by:** faithful-determined; **Owner-confirmed** via the Phase 2 plan
  sign-off.
- **Falsification condition:** gameplay needs laser travel-time/leading, or a
  muzzle-ray diverges noticeably from the on-screen crosshair → switch to a
  screen-space pick.

## DL-013 — Missiles = homing projectile entities · ACCEPTED (Phase 2)

- **Context:** unlike lasers, missiles are real moving objects that home on a
  locked target and can be destroyed in flight by ECM (`GAMEPLAY.md` §10).
- **Alternatives considered:** instant "damage-after-delay" (can't be ECM'd in
  flight — unfaithful); hitscan missiles (no travel — unfaithful).
- **Decision:** spawn a missile entity carrying a **project-defined**
  `Velocity(Vec3)`, a `Ttl(Timer)`, and `Homing { target: Entity }`; it steers
  toward the target each frame and is despawned on impact, on ECM, or on TTL
  expiry (`try_despawn` to tolerate the hit-and-expire race).
- **Rationale:** faithful homing + ECM counterplay with a trivial integrator.
- **Decided by:** faithful-determined; **Owner-confirmed.**
- **Note:** `Velocity` is a project type, **not** a Bevy/physics-crate component.
- **Falsification condition:** missile volume makes per-entity homing too costly
  → pool entities or simplify the integrator.

## DL-014 — Collision = per-frame sphere-sphere, no physics crate · ACCEPTED (Phase 2)

- **Context:** ship-ship and missile-ship contact; the same radius feeds the
  DL-012 laser test.
- **Alternatives considered:** a physics crate (`avian`/`rapier`) — a heavy
  dependency, overkill for the handful of NPCs in a system; AABB boxes (worse fit
  for ships than spheres).
- **Decision:** every targetable carries `CollisionRadius(f32)`; each frame,
  candidate pairs are compared with `distance_squared ≤ (ra + rb)²` (no `sqrt`),
  via a linear scan. `bevy_math` supplies the primitives but **no broadphase, no
  Transform→volume sync, and no collision events** — that loop is ours to write.
- **Rationale:** minimal dependencies, faithful enough, trivial at Phase-2 entity
  counts.
- **Decided by:** faithful-determined; **Owner-confirmed.**
- **Falsification condition:** entity counts grow until O(n²) hurts → add a
  spatial grid (there is no built-in spatial index).

## DL-015 — Enemy AI = full faithful tactics routine · ACCEPTED (Phase 2)

- **Context:** NPC combat behaviour. `GAMEPLAY.md` §12: dot-product
  nose-vs-bearing aim under the no-yaw model (DL-011), energy-threshold weapon
  choice, ~10% escape-pod bail, ECM use, Thargon release.
- **Alternatives considered:** a minimal Approach→Attack→Evade FSM now with the
  faithful nuances deferred to a later sub-phase.
- **Decision:** implement the **full faithful tactics routine** in Phase 2 — the
  Approach/Attack/Evade states *plus* energy-based laser/missile choice, the
  bail-out probability, ECM, and (for Thargoids) Thargon release.
- **Rationale:** the Owner wants maximal faithfulness now; the tactics are fully
  specified in `GAMEPLAY.md`.
- **Decided by:** **user-approved** (Phase 2 plan options: full tactics vs. simple
  FSM → full tactics).
- **Falsification condition:** the full routine proves too tangled to tune/test as
  one unit → factor it into layered, independently-testable behaviours.

## DL-016 — Evade exit = hybrid (distance goal + timeout cap) · ACCEPTED (Phase 2)

- **Context:** when an evading enemy turns back to re-engage. A bare timer and a
  bare spatial test each have a failure mode (see Phase 2 plan).
- **Alternatives considered:** timer only (ungrounded magic number; re-engages
  even while still close); spatial only (loops away forever from a fast player).
- **Decision:** evade until it has opened a target distance from the player **OR**
  a timeout fires, whichever comes first.
- **Rationale:** the distance goal handles the normal case; the timeout caps the
  pathological "chased forever" case. One well-defined rule, two tuned values.
- **Decided by:** **user-approved** (Phase 2 plan options: timer vs. spatial vs.
  hybrid → hybrid).
- **Falsification condition:** playtesting shows one condition always dominates →
  collapse to the dominant one.

## DL-017 — Phase-2 enemy spawning = debug command-spawn + in-system design hook · ACCEPTED (Phase 2)

- **Context:** how enemies enter the world for the combat slice, before the
  universe and legal-status systems exist (Phase 3–4).
- **Alternatives considered:** debug-only spawn (no forward design — risks
  reworking combat at Phase 3); in-system procedural spawn only (needs context
  that doesn't exist yet).
- **Decision:** ship a **debug command-spawn** (a key spawns an enemy ahead of the
  player, **offset off the −Z station axis** to avoid the ring-station overlap an
  earlier review flagged), **and** define the **in-system spawn contract**
  (faction, component bundle, trigger) so the Phase 3 procedural spawner plugs in
  cleanly without touching combat internals.
- **Rationale:** full control for testing combat now; a designed seam keeps
  Phase 3 from reworking Phase 2.
- **Decided by:** **user-approved** (Phase 2 plan options: debug vs. in-system vs.
  both → both).
- **Retirement condition:** the debug command-spawn is removed when a real
  encounter-spawn system ships (procedural in-system or mission-scripted),
  regardless of phase — a named-successor trigger (cf. DL-009).

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
