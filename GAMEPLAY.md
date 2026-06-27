# GAMEPLAY.md — faithful gameplay specification (1984 *Elite*)

This document describes how the original 1984 **Elite** plays, in enough detail to
drive a faithful remake. It is the **design baseline**: every later phase
(combat, universe, economy, stations, progression) implements the mechanics
recorded here. `DESIGN.md` holds *our* architecture and phase roadmap;
`DECISIONS.md` records the load-bearing forks. This file holds *what the game is*.

> **Status of the numbers.** The data tables here were researched against
> primary sources and cross-checked by an adversarial verification pass. Where a
> figure is confirmed against the disassembled 6502 source (bbcelite.com), the
> Elite wiki (wiki.alioth.net), or the boxed *Space Trader's Flight Training
> Manual*, it is stated plainly. Where a figure is widely repeated but not
> source-confirmed, it is marked **(unverified)**. An errata note appears wherever
> a commonly-cited "fact" is wrong.

---

## 1. Mandate and scope

**Faithful to the 1984 original.** The target is *Elite* as released by Acornsoft
in September 1984 on the BBC Micro and Acorn Electron (David Braben & Ian Bell).
The remake reproduces its **systems** — procedural galaxy, supply/demand economy,
real-time wireframe-style combat, ship/equipment progression, combat & legal
ranks — while modernising only *presentation* (solid 3D rendering, modern input).

**Explicitly out of scope (sequel mechanics — do not import):**

- **Newtonian flight / inertia**, **yaw** as a free axis, **time compression**,
  **seamless planetary landing**, a **single Milky-Way-scale galaxy**,
  **multiplayer**, **on-foot/SRV** — all of these are **Frontier: Elite II (1993)**,
  **First Encounters (1995)**, or **Elite Dangerous (2014)**. None belong here.
- Sequel-era numbers that leak into web searches (e.g. the "222 m" Coriolis slot,
  Elite Dangerous bounty-as-credits, Oolite "Tech Shuffle" equipment levels) are
  **not** the 1984 game and must not be used as sources.

---

## 2. Version baseline — **DECIDED: BBC disc / enhanced superset** (DL-010)

The "1984 game" is not a single artifact. Features differ across the cassette,
the BBC **disc** release, the home-computer **ports** (C64, Spectrum, Amstrad,
NES, Archimedes), and the **Acorn Electron** cut-down. This choice cuts across
*every* subsystem (the buy list, mining, missions, the bestiary, even the
docking computer), so it must be decided once.

| Feature | BBC cassette | BBC disc / enhanced | Acorn Electron | Ports (C64/NES/…) |
|---|---|---|---|---|
| Pulse + Beam lasers | ✅ | ✅ | ✅ | ✅ |
| **Military / Mining lasers (buyable)** | ❌ | ✅ (enhanced) | ❌ | ✅ |
| **Asteroid mining + Rock Hermit** | ❌ | ✅ | ❌ | ✅ |
| **Missions** (Constrictor, Thargoid Plans) | ❌ none | ✅ | ❌ | ✅ (+ port-only missions) |
| Thargoids / suns | ✅ | ✅ | ❌ (no suns, no Thargoids) | ✅ |
| Docking computer | instant | animated (DOCKIT) | instant | animated + "Blue Danube" gag |
| Continuous music | ❌ | ❌ | ❌ | ✅ (C64, NES) |
| Trumbles | ❌ | ❌ | ❌ | C64 + NES only |

**Decision (DL-010): the BBC *disc / enhanced* superset is canonical.** It is the
fullest expression of the game on its native hardware: it has the missions,
mining, and the full laser line-up, without the port-only extras (Blue Danube,
Trumbles, soundtracks, extra missions) that are not "the BBC original." A
`cassette-pure` mode (pulse+beam only, no missions, no mining) is a later toggle.

---

## 3. World & fiction

- You are a lone trader-combatant flying a **Cobra Mk III**, starting at the
  Coriolis station orbiting planet **Lave** in **Galaxy 1** with **100 Cr**, a
  single front pulse laser, 3 missiles, and a full 7-light-year fuel tank.
- The setting is the **Galactic Cooperative of Worlds (GalCop)**: 8 galaxies of
  256 systems, each with its own economy, government, and tech level. Police
  **Vipers** enforce the law inside station safe-zones; **pirates** prey on
  traders in lawless space.
- The recurring threat is the **Thargoids** — insectoid aliens whose warships
  ambush ships in **witchspace**, deploying robotic **Thargon** drones.
- **The Dark Wheel** (Robert Holdstock novella, bundled with the boxed game)
  supplies backstory. *Errata:* its protagonist is **Alex Ryder, son of Jason
  Ryder** — **not** "Commander Jameson" (which is only the default in-game
  save name). The novella's canon status is itself disputed.
- **No win condition.** The sandbox is the game; the implicit goal is to climb the
  combat ladder to **Elite**.
- The intended universe was reportedly **248 galaxies**, cut to 8 by Acornsoft to
  hide the procedural origins. (The oft-quoted "282 trillion" figure is
  unsupported — do not cite it.)

---

## 4. Global conventions (NORMATIVE)

These conventions thread every subsystem. State them once; honour them everywhere.

- **Fixed-point money.** All prices and cash are stored as **10× the displayed
  value** and shown to one decimal place. Cash, commodity prices, equipment
  prices, and bounties all follow this. (Internally, commodity price uses a `×4`
  step; bounties are 16-bit tenths-of-a-credit.)
- **Fuel** is stored in **tenths of a light year**, range **0–70** (full tank =
  7.0 LY). Refuel costs **0.2 Cr per 0.1 LY** (a full top-up ≈ 14 Cr).
- **Internal spatial units** are abstract (not km). For scale: per-system
  coordinate cube ±8,388,607 per axis; ships vanish beyond ~57,344 from the
  player; planet radius 24,576; station sits 65,536 from the planet centre; the
  sun spawns 65,536–458,752 out.
- **Tick basis.** The original ran a fixed main loop at the **50 Hz** display
  refresh. Per-tick rates (laser fire counter, energy recharge "every 8
  iterations", overheating, FIST decay per jump, Trumble breeding) are all tied to
  that loop. **The remake must re-express every per-tick rate as a time-based
  (`dt`-scaled) rate** so behaviour is frame-rate independent — record the
  original loop timing alongside each constant.
- **One planet + one sun + one station per system** (plus a possible Rock Hermit
  in mining-capable versions). The original does **not** model multi-body systems
  — do not over-model them like the sequels.

---

## 5. The core gameplay loop

```
        ┌─────────────────────── dock ───────────────────────┐
        ▼                                                     │
   [ STATION ]  buy/sell cargo · refuel · buy equipment       │
        │       · (accept mission) · launch                   │
        ▼                                                     │
   [ IN-SYSTEM FLIGHT ]  fly from the witchpoint to the       │
        │  station; trade routes, pirates, police, asteroids; │
        │  mine, fight, scoop cargo & fuel                     │
        ├──────────── hyperspace (≤7 LY, costs fuel) ──────────┘
        ▼
   [ NEW SYSTEM ]  arrive at the witchpoint (risk of Thargoid
                   ambush in witchspace) → repeat
```

Real-time throughout (not turn-based). Progress is measured by **credits**,
**combat rank**, and **equipment** accumulated.

---

## 6. Galaxy generation — the seed model

The entire universe is **procedurally generated from a tiny seed**, not stored.
**8 galaxies × 256 systems = 2,048 systems**, identical for every player.

- Each galaxy is described by **three 16-bit seeds** `(s0, s1, s2)` — six bytes,
  little-endian. They are three consecutive terms of a Tribonacci-style sequence.
- **The twist** (advance one step): `s0' = s1`, `s1' = s2`,
  `s2' = s0 + s1 + s2` (16-bit, wrapping).
- **Per system:** apply the twist **4 times**. System *N*'s seeds = galaxy-start
  seeds twisted `4 × N` times. Walking all 256 systems = 1024 twists, then it
  wraps back to system 0.
- **Galactic hyperdrive:** roll each of the six seed bytes **left by one bit**
  (8-bit ROL). Eight rolls cycle Galaxy 1 → 2 → … → 8 → 1.

### Seed → system attribute map  *(corrected — see errata)*

| Attribute | Source bits | Range | Notes |
|---|---|---|---|
| **Galactic x** | **`s1_hi`** (full byte) | 0–255 | Lave x = 20 |
| **Galactic y (displayed)** | **`s0_hi >> 1`** | 0–127 | Lave: byte 173 → plotted 86 |
| Economy | `s0_hi` bits 0–2 | 0–7 | bit 1 forced set if gov is Anarchy/Feudal |
| Government | `s1_lo` bits 3–5 | 0–7 | |
| Tech level | `flip(econ) + (s1_hi AND 3) + (gov >> 1)` | 0–14 (shown 1–15) | |
| Population | `(tech × 4) + econ + gov + 1` | 1–71 | = 0.1–7.1 billion |
| Productivity (GNP) | `(flip(econ) + 3) × (gov + 4) × pop × 8` | 96–62,480 | M Cr |
| Avg planet radius | `((s2_hi AND 15) + 11) × 256 + s1_hi` | 2,816–6,911 | km (data screen) |
| Name length | `s0_lo` bit 6 | 6 or 8 chars | set ⇒ 4 digram pairs |
| Species human? | `s2_lo` bit 7 | clear ⇒ "Human Colonials" | |

> **ERRATA (load-bearing).** A first-pass research claim that galactic
> **x = `s1_lo`** and **y = `s0_hi`** is **WRONG** and would put every system at
> the wrong place. The canonical bbcelite "Generating system data" page and the
> chart routine (`LDA QQ15+3` = `s1_hi` → x; `LSR QQ15+1` = `s0_hi`>>1 → y) give
> **x = `s1_hi`, y = `s0_hi >> 1`**. Lave's `s1 = 0x149C`, so `s1_hi = 0x14 = 20`
> (the canonical x), not `s1_lo = 0x9C = 156`.
> **Encode x = `s1_hi`, y = `s0_hi >> 1`** and keep the y-halving in the distance
> metric. A golden-file test over all 2,048 systems is strongly recommended.

### Economy & government types

| Econ | Type | | Gov | Type |
|--:|---|---|--:|---|
| 0 | Rich Industrial | | 0 | Anarchy |
| 1 | Average Industrial | | 1 | Feudal |
| 2 | Poor Industrial | | 2 | Multi-Government |
| 3 | Mainly Industrial | | 3 | Dictatorship |
| 4 | Mainly Agricultural | | 4 | Communist |
| 5 | Rich Agricultural | | 5 | Confederacy |
| 6 | Average Agricultural | | 6 | Democracy |
| 7 | Poor Agricultural | | 7 | Corporate State |

Anarchy/Feudal governments force economy bit 1 set → they can never be rich.
Law enforcement scales with government: anarchies have **no police**; corporate
states are strictest.

### Procedural text

- **System names** are built from a 64-entry **digram** (two-letter token) table
  (QQ16), tokens 128–159: `AL LE XE GE ZA CE BI SO US ES AR MA IN DI RE A? ER AT
  EN BE RA LA VE TI ED OR QU AN TE IS RI ON`. Bit 6 of `s0_lo` selects a 3- or
  4-pair (6/8-letter) name; `A?` emits a single trailing letter (odd-length
  names).
- **System descriptions** ("goat soup") expand a recursive token tree seeded from
  `s1`/`s2` (top template = token 5 → `<adjective> IS <noun-phrase>`). Per-port
  wording differs; full verbatim reproduction is a later task (flag the table
  incomplete until fully extracted).

Galaxy 1 start seeds: `s0=0x5A4A, s1=0x0248, s2=0xB753` (system 0 = Tibedied;
Lave is the start system). Galaxies 3–8 follow from the byte-ROL.

---

## 7. Navigation & hyperspace

- **Two charts:** the **Short-Range Chart** (local systems, with a circle showing
  how far current fuel reaches) and the **Galactic Chart** (all 256 systems).
  "Find planet by name" snaps the cursor to the nearest match and shows distance.
- **Distance metric:** `dist = sqrt(dx² + (dy/2)²)` (the y-axis is half-weighted
  because the map is twice as wide as tall); stored ×4 internally. Map scale ≈
  0.4 LY per x-step, 0.2 LY per y-step.
- **Hyperspace:** select a target **within remaining fuel**; jump cost =
  `distance(LY) × 0.1`. Max single jump = **7.0 LY**. Targets beyond fuel/range
  are rejected ("too far" / no fuel). A countdown precedes the jump
  (~15 s, **unverified**).
- **Witchpoint arrival.** You exit hyperspace at an **in-system witchpoint** a
  fixed game-distance from the planet, and cruise in under normal drive.
  *Errata:* the witchpoint is **not** "7 LY from the planet" — that conflates
  arrival distance with jump range.
- **Galactic hyperdrive:** one-shot, **5,000 Cr**, jumps to the next of the 8
  galaxies. *Errata:* the BBC/Electron **cassette** shipped with a bug that broke
  galactic hyperspace (fixed in v1.1).
- **Witchspace / Thargoid ambush:** a jump can be intercepted, dropping you into
  empty witchspace to fight Thargoids. A **deliberate misjump** (targeting the gap
  between systems / beyond range) also drops you into witchspace. Interception
  probability and the "interrupted jump still burns full fuel" claim are
  **(unverified)** against the 1984 source — flag before relying on them.

---

## 8. Flight model, controls & camera

- **Two rotational axes only: PITCH and ROLL, plus throttle.** **The Cobra has
  NO yaw.** You aim by *rolling* a target into the vertical (pitch) plane, then
  *pitching* onto it. This is a defining trait of classic Elite and shapes combat,
  AI, and docking.
- **No Newtonian inertia.** There is a top speed; releasing thrust slows you.
  Turning is rate-based (hold to rotate, release to stop).
- **Speed numbers** *(don't conflate):* the Cobra blueprint stat is **28**, but
  the **player's operational max speed is the `DELTA` variable, max 40**; the
  manual status page shows "0.30 LM". The speed bar is 0..32 px representing
  0..40. Use **40** as the player's operational max.

### Controls (BBC cassette, from the `KYTB` table)

| Key | Action | | Key | Action |
|---|---|---|---|---|
| `<` / `>` | Roll left / right | | `A` | Fire laser |
| `X` / `S` | Pitch up / down | | `T` / `U` / `M` | Target / un-arm / fire missile |
| `SPACE` / `?` | Speed up / down | | `E` | E.C.M. |
| `f0`–`f3` | Front / rear / left / right view | | `TAB` | Energy bomb |
| `J` | In-system jump | | `ESCAPE` | Launch escape capsule |
| `H` | Hyperspace | | `C` | Docking computer (enhanced/disc) |

> The **four views are also the four laser mounts.** A laser fires in whichever
> direction you are looking; equipment is bought per-view. (C64 remaps: views
> `F1/F3/F5/F7`, roll `,`/`.`, dock `C`/`P`, galactic `CTRL+H`.)

- **In-system "J" jump:** fast travel that **aborts** when mass (a station,
  planet, or ship) is near — the original's anti-collision safeguard.
- **Camera:** first-person cockpit. The remake mounts the camera on the ship
  (already done in Phase 1).

> **Remake stance — yaw (DECIDED: DL-011).** Phase 1 originally shipped
> **roll + pitch + yaw** (A/D = yaw); the 1984 original has **no yaw**. The
> resolved decision is a **hybrid**: faithful **no-yaw is the default**
> (**W/S = pitch, A/D = roll, R/F = throttle**), with an optional **"yaw assist"**
> mode behind a toggle (key `Y`) that re-enables yaw on **Q/E**. The damped-arcade
> integration model (DL-004) is unchanged — only the available axes change.

---

## 9. HUD & dashboard

The dashboard is a fixed instrument panel. Indicators and their internal ranges:

| Indicator | Internal range | Notes |
|---|---|---|
| Speed bar | 0..32 px | represents 0..40 (`DELTA`) |
| Fuel | bar 0..64 px | **value 0..70** (= 7.0 LY) |
| Energy banks (×4) | first bank **0..15**, others **0..16** | total survivability |
| Fore shield (FS) / Aft shield (AS) | 0..255 | absorb hits before energy |
| Cabin temp (CT) | 0..255 | 30 in deep space; rises near the sun |
| Laser temp (LT) | 0..255 | overheats > 242 |
| Altitude (AL) | 0..255 | over the planet |
| Roll (RL) / Pitch (DC) | centred bars | rate indicators |
| `E` / `S` lights | — | ECM active / inside station safe-zone |
| Condition | GREEN / YELLOW / RED | green = safe zone, red = hostile near |

- **3D scanner ("compass radar"):** an elliptical scope showing nearby objects as
  blips with a **vertical height-stalk** encoding elevation above/below your
  plane. A separate **compass** points to the planet (then the station when in
  range), bright when ahead / dim when behind.

> *(Two-letter glyphs SP/FS/AS/FU/CT/LT/AL come from the manual; the centred-bar
> "16 positions, centre 8" figure and the exact CONDITION rendering are
> **unverified** — pin to source during implementation.)*

---

## 10. Combat — weapons

- **Lasers** mount on any of the four views. Types:
  - **Pulse** — `POW = 15`, fire counter `LASCT = 10` ⇒ ~5 shots/sec at 50 Hz.
  - **Beam** — `POW = 15 + 128 = 143` (bit 7 set ⇒ continuous), `LASCT = 0`
    (fires every refresh). Hotter, faster.
  - **Mining** — `POW = 50` **(unverified)**; cracks asteroids into splinters.
    *Enhanced/disc & ports only.*
  - **Military** — `POW = 151` **(unverified)**; the strongest. *Enhanced/disc &
    ports only.*
  - **Overheat:** any laser cuts out when laser temperature `GNTMP > 242`, until
    it cools.
- **Hit detection:** a target is hit when the **squared** crosshair distance is
  within the ship's stored **targetable area** (Cobra Mk III = radius **95**,
  stored as `95×95 = 9025`). Instant hit, no projectile travel. The remake should
  reproduce this as a raycast/cone, preserving the no-travel feel.
- **Missiles:** up to **4** carried. `T` locks the target (needs a lock tone),
  `M` fires, `U` un-arms. **ECM** (`E`) destroys incoming *and* your own in-flight
  missiles in a radius; a point-blank launch can beat the enemy's ECM reaction.
- **Energy bomb:** one-shot (`TAB`), screen-flash, **destroys nearby small ships,
  asteroids, and missiles.**
  > **ERRATA (load-bearing, version-specific).** In the **1984 original** the
  > energy bomb **CAN destroy Thargoids.** Thargoid *immunity* to the energy bomb
  > was introduced in the **BBC Master (1986)** — a later port, not the 1984 game.
  > A faithful 1984 baseline lets the energy bomb kill Thargoids. (Station
  > immunity is plausible but not verbatim-confirmed.)

---

## 11. Combat — defence & damage

- **There is no separate hull bar — energy *is* survivability.** Incoming fire
  drains the relevant **shield** (fore shield for frontal hits, aft for rear),
  then the **4 energy banks**; reaching zero energy = death.
- **Damage model (`OOPS`):** an NPC hit subtracts roughly **half the attacker's
  laser-power byte**; a large enough hit can punch through to **cargo loss** or
  death. Hit hemisphere selects which shield drains first.
- **Recharge cadence:** every **8 main-loop iterations**, energy **+1**
  (**+2** with the Extra Energy Unit; **+3** with the mission-only Naval Energy
  Unit). Shields top up from energy **only while energy is above ~50%**. Max
  energy = **150**.
- **Player escape capsule** (`ESCAPE`): launches you to safety; you **lose all
  cargo and special mission items**, keep insured equipment (you respawn in an
  identically-equipped Cobra at the last station), and your **legal status resets
  to Clean**. (Trumble gag: one Trumble survives the pod.)
- **Cabin temperature** rises near the sun; lingering at max destroys the ship.
  Fuel scoops engage at **CT = 224** (sun-skimming) and refill the tank for free;
  they also scoop cargo canisters and asteroid splinters.

---

## 12. NPCs, AI & bestiary

- **Roles are behaviours, not separate ships:** the same hull can be a **trader**,
  **pirate**, **bounty hunter**, or **GalCop police Viper** depending on state.
  Police spawn in response to your **legal status** (see §14) and to attacks
  inside the station safe-zone.
- **Tactics routine (per NPC):** chooses laser vs missile (~50% energy
  threshold), ~10% chance to **bail out in an escape pod** when losing, uses a
  **dot-product nose-vs-bearing** test to "turn toward / away" under the
  **2-axis (no-yaw)** attitude model, triggers ECM against your missiles, and a
  Thargoid releases **Thargons** in place of missiles.
- **Bounties** are paid for kills; a kill also adds to your combat rank (see §15).
  Destroyed ships may drop **cargo canisters** to scoop.

### Bestiary — canonical stat table

> Two source tables exist and **disagree by version** (BBC disc "firepower" vs the
> C64 blueprint dump). Per the §2 baseline (BBC disc superset), use the **BBC
> disc** column as canonical; the C64 numbers are recorded in the appendix for
> reference. Pick one set and use it uniformly across combat, AI, and equipment.

| Ship | Hull | Laser cls | Missiles | Bounty (Cr) | Role |
|---|--:|--:|--:|--:|---|
| Sidewinder | 70 | 2 | 0 | 5.0 | pirate (cheap) |
| Gecko | 70 | 2 | 0 | 5.5 | pirate |
| Krait | 80 | 2 | 0 | 10.0 | pirate |
| Adder | 85 | 2 | 0 | 4.0 | trader/pirate |
| Cobra Mk I | 90 | 2 | 2 | 7.5 | trader |
| Mamba | 90 | 2 | 2 | 15.0 | pirate (agile) |
| Moray | 100 | 2 | 0 | 5.0 | aquatic trader |
| **Viper** | 100 | 2 | 1 | — | **GalCop police** |
| Asp Mk II | 150 | 5 | 1 | 20.0 | fighter |
| **Cobra Mk III** | 150 | 2 | 3 | 17.5 | **player ship** |
| Fer-de-Lance | 160 | 2 | 2 | — | smuggler |
| **Thargoid** | 240 | 2 | 6 | 50.0 | alien warship |
| Boa | 250 | 3 | 4 | — | large trader |
| Python | 250 | 3 | 3 | 20.0 | bulk trader |
| Anaconda | 252 | 7 | 7 | — | freighter (huge) |
| **Constrictor** | 252 | 6 | 4 | — | mission target |

Non-combat actors: Thargon (drone), Worm, Shuttle, Transporter, Missile,
Escape pod, Cargo canister, Asteroid (splits into splinters), Rock Hermit.

---

## 13. Economy & trading

- **17 commodities.** Local price and available quantity depend **only on the
  system's economy type and a per-system random fluctuation byte (`QQ26`)** —
  **not** on government or tech level. `QQ26` re-rolls **only on hyperspace
  arrival** (stable within a visit; the market screen recomputes from it each
  time it opens).
- **Price formula** (stored value = 10× display):
  `price = (base_price + (rand AND mask) + economy × factor) × 4`
- **Availability** (clamped 0..63): `qty = base_quantity + (rand AND mask) −
  economy × factor`; negative → 0.
- **Buying** reduces local stock and your cash, adds to cargo; **selling** dumps
  your **whole holding** of that good.
- **Arbitrage loop:** industrial systems are cheap in **Computers/Machinery/
  Firearms** and dear in **Food/Textiles**; agricultural systems are the reverse.
  Buy Computers in industrial → sell in agricultural; buy Food in agricultural →
  sell in industrial. Profits are modest and compounding.
- **Cargo hold:** 20 t standard, **35 t** with the Large Cargo Bay (the 35 t
  figure is widely cited but **unverified** against the BBC source).

### QQ23 — master commodity table (BBC cassette, exact)

| Commodity | base | factor | unit | base qty | mask | Illegal |
|---|--:|--:|:--:|--:|---|:--:|
| Food | 19 | −2 | t | 6 | &01 | |
| Textiles | 20 | −1 | t | 10 | &03 | |
| Radioactives | 65 | −3 | t | 2 | &07 | |
| **Slaves** | 40 | −5 | t | 226 | &1F | **yes** |
| Liquor/Wines | 83 | −5 | t | 251 | &0F | |
| Luxuries | 196 | +8 | t | 54 | &03 | |
| **Narcotics** | 235 | +29 | t | 8 | &78 | **yes** |
| Computers | 154 | +14 | t | 56 | &03 | |
| Machinery | 117 | +6 | t | 40 | &07 | |
| Alloys | 78 | +1 | t | 17 | &1F | |
| **Firearms** | 124 | +13 | t | 29 | &07 | **yes** |
| Furs | 176 | −9 | t | 220 | &3F | |
| Minerals | 32 | −1 | t | 53 | &03 | |
| Gold | 97 | −1 | kg | 66 | &07 | |
| Platinum | 171 | −2 | kg | 55 | &1F | |
| Gem-Stones | 45 | −1 | g | 250 | &0F | |
| Alien Items | 53 | +15 | t | 192 | &07 | |

`factor` sign is byte-1 bit 7; the unit (t/kg/g) is encoded in byte-1 bits 5–6.

> **ERRATA.** "Average price" tables that quote **Slaves 8.0 / Firearms 70.4 /
> Narcotics 114.8** as *base prices* are **wrong** — those are illustrative
> manual samples, not the QQ23 bytes. Derive all prices from the formula above.
> *(NES port renamed Slaves → "Robot Slaves", Narcotics → "Rare Species".)*

---

## 14. Contraband & legal status (FIST)

- **Three illegal goods:** **Slaves, Narcotics, Firearms.** Carrying or trading
  them, attacking traders/police, or killing the innocent raises your **`FIST`**
  (offence) byte.

| Status | FIST | Effect |
|---|--:|---|
| Clean | 0 | police ignore you |
| Offender | 1–49 | wanted; police hostile; can still dock |
| Fugitive | ≥ 50 | hunted on sight (NES port: boundary is **40**) |

- **Evaluation:** FIST is assessed **at launch** from a station (it accumulates
  from contraband carried and actions taken). Rough per-tonne increments: ~+2/t
  for slaves & narcotics, ~+1/t for firearms (you turn Fugitive at ~25 t
  slaves/narcotics or ~50 t firearms). *(per-action increments are **unverified**.)*
- **Decay:** modelled as **FIST halved per jump** (right-shift) — well attested
  for Oolite, **not source-confirmed for the 1984 original**; document it as
  "modelled on the original."
- **Three instant resets to Clean:** the **escape capsule**, a **galactic
  hyperspace** jump, and **buying a new ship**.
- **No literal player credit-bounty** in the 1984 game (that's a sequel concept);
  wanted-ness is the FIST *status* byte. Bounties *paid to you* for kills are real.
- **Architectural note:** the economy plugin should **emit a "carrying/sold
  contraband" event**; the combat/AI plugin decides police behaviour. Do not
  hard-couple them.

---

## 15. Progression & combat ranks

Kills accumulate in a 16-bit counter (`TALLY`). In the **BBC versions every kill
= 1 point** (even asteroids/canisters); later ports use fractional per-ship
values.

| Rank | Kills to reach |
|---|--:|
| Harmless | 0 |
| Mostly Harmless | 8 |
| Poor | 16 |
| Average | 32 |
| Above Average | 64 |
| Competent | 128 |
| Dangerous | 512 |
| Deadly | 2,560 |
| **Elite** | **6,400** |

Every **256 kills** the game flashes the famous **"RIGHT ON COMMANDER!"**.
Reaching **Elite** confers prestige only — no mechanical reward. You start
**Harmless / Clean / 100 Cr** at Lave.

---

## 16. Player ship & equipment

### Cobra Mk III (fixed blueprint)

| Stat | Value |
|---|---|
| Cargo hold | 20 t (35 t with Large Cargo Bay — *unverified*) |
| Starting cash | 100.0 Cr |
| Starting laser | 1 × front Pulse |
| Starting missiles | 3 (max 4) |
| Fuel / max jump | 7.0 LY |
| Max energy | 150 |
| Blueprint max speed / player max | 28 / **40** (`DELTA`) |
| Laser power (base) | 2 |
| Targetable area | 95 (stored 9025) |
| Canisters on demise | up to 3 |
| Geometry | 28 vertices, 13 faces, 38 edges |

### Equipment catalogue (BBC source `PRXS`; prices in Cr)

| # | Item | Price (Cr) | Notes |
|--:|---|--:|---|
| 1 | Fuel | 0.2 / 0.1 LY | full top-up ≈ 14.0 |
| 2 | Missile | 30.0 | max 4 |
| 3 | Large Cargo Bay | 400.0 | 20 t → 35 t |
| 4 | E.C.M. System | 600.0 | reusable |
| 5 | Pulse Laser | 400.0 | per view |
| 6 | Beam Laser | 1,000.0 | per view; pulse refunded |
| 7 | Fuel Scoops | 525.0 | sun-skim + scoop |
| 8 | Escape Capsule | 1,000.0 | one-shot; clears record |
| 9 | Energy Bomb | 900.0 | one-shot |
| 10 | Extra Energy Unit | 1,500.0 | +2 recharge |
| 11 | Docking Computer | **1,000.0** (code) / 1,500 (manual) | code is authoritative |
| 12 | Galactic Hyperdrive | 5,000.0 | one-shot |
| — | Mining Laser | 800.0 | **enhanced/disc & ports only** |
| — | Military Laser | 6,000.0 | **enhanced/disc & ports only** |

### Tech-level gating — **use the code formula, not the manual table**

The shop shows items `1..N` where **`N = min(tech_level + 3, 12)`** against the
**fixed menu order** above. So an item with menu number `k` is on sale when
`k ≤ tech + 3`:

| Item | On sale at tech ≥ | (manual prints) |
|---|--:|--:|
| ECM | 1 | 2 |
| Pulse | 2 | 3 |
| Beam | 3 | 4 |
| Fuel Scoops | 4 | 5 |
| Escape Capsule | 5 | 6 |
| Energy Bomb | 6 | 7 |
| Extra Energy Unit | 7 | 8 |
| Docking Computer | 8 | 9 |
| Galactic Hyperdrive | 9 | 10 |

> **ERRATA.** The boxed manual's per-item tech table is **one level higher** at
> every row than the actual game code. For a faithful sim, implement
> `N = min(tech+3, 12)`. (Mining/Military lasers are appended by a separate
> enhanced-version code path, gated ~TL10 per the manual; the disc code path was
> not re-read — treat the exact gate as **unverified**.)

---

## 17. Missions (enhanced/disc content)

| Mission | Trigger | Reward | Notes |
|---|---|---|---|
| **Constrictor** | Competent **and ≥ 256 total kills**, in **Galaxy 1 or 2** | **5,000 Cr + 256 kill pts** | briefed by Capt. **Carruthers** (G1) / Capt. **Fosdyke Smythe** (G2). The Constrictor is energy-bomb-proof — kill it with lasers. **NO energy unit reward.** |
| **Thargoid Plans** | After Constrictor; ~3/8 Dangerous→Deadly; **Galaxy 3 only** (Ceerdi → Birera) | **Naval Energy Unit (×3 recharge) + 256 kill pts** | ~22% Thargoid spawn while carrying the plans. **NO cash reward.** |
| Trumbles | Cash reaches threshold (C64 5,017.6 / NES 6,553.6 Cr) | comedy breeding pest | **buy for 5,000 Cr**; survives the escape pod; remove by flying near the sun. **C64 & NES only.** |
| Cougar / Supernova / Thargoid Invasion | various | — | **port-only**, out of the 1984 baseline |

> **ERRATA.** The Naval Energy Unit (×3 recharge) comes **only** from Thargoid
> Plans. The Constrictor pays cash only. The buyable **Extra** Energy Unit is the
> ×2 tier. Recharge tiers: base ship **1** → Extra Energy Unit **2** → Naval
> Energy Unit **3**.

---

## 18. Stations & docking

- **Coriolis station:** a **cuboctahedron** (a cube with its corners cut — *not*
  an octahedron), slowly rotating, with a single **rectangular docking slot** on
  the planet-facing side.
- **Manual docking** (the `ISDK` checks): the station must be **friendly**, you
  must be **facing the slot** (within ~26°), **heading toward** it, inside a
  ~22° **approach cone**, and **roll-matched** to the slot (roof-vector x ≥ 80,
  ~33.6° tolerance). Fail any → you crash and explode. (A low-speed requirement is
  commonly stated but **unverified** in source.)
- **Docking Computer:** cassette = **instant** guaranteed dock; disc/enhanced =
  **animated** `DOCKIT` approach. (The "Blue Danube" docking music is a
  **C64/port** gag, not on the BBC.) The classic auto-docker can occasionally
  crash you into the station — the infamous "docking computer of death".

---

## 19. Audio & presentation

- **BBC original:** sparse sound effects and a short title fanfare; **no
  continuous music** in flight.
- **Ports:** C64 has a title theme by **Aidan Bell / Julie Dunn** (driver by Dave
  Dunn) plus the Blue Danube docking tune; NES music by David Whittaker.
  *Errata:* **Rob Hubbard did NOT compose C64 Elite** — a common misattribution.
- **Rendering:** the original is **wireframe** (C64 used sprites; Archimedes
  ArcElite used filled polygons). The remake uses **solid 3D** but must preserve
  **gameplay-load-bearing proportions** — seed-derived planet radius, the
  cuboctahedron slot geometry, the targetable-area hit test, ship visibility
  distance, and the 2D scanner's height-stalk projection. Modernising *shading* is
  fine; changing *proportions*, or turning the charts into a 3D starmap (which
  would break the seed-coordinate model), is not.

---

## 20. Cross-cutting invariants

These couplings must stay consistent across plugins:

- **The seed model is the single source of truth** for galaxy layout, economy
  inputs, mission locations, and the start system — use the corrected
  `x = s1_hi, y = s0_hi >> 1`. Golden-file test all 2,048 systems.
- **Tech level** is seed-derived, gates equipment via `min(tech+3,12)`, and must
  use the **same** value for gating and for the data screen. Prices are **never**
  coupled to tech/government.
- **FIST** couples economy (contraband) → combat (police spawns) → navigation
  (per-jump decay, galactic-jump reset). Route it through events.
- **Energy/shields** is shared state between the HUD (display, recharge cadence),
  combat (damage absorption, death), and equipment (Extra/Naval Energy Unit).
  There is no separate hull.
- **The no-yaw model** couples flight, combat aiming, NPC AI, and the docking
  roll-match. Changing it (adding yaw) ripples through all four.
- **The fixed-point money convention** (store 10×) and the **fuel model** (tenths,
  0–70) thread every numeric table.
- **Frame-rate independence:** re-express every per-50Hz-tick rate as a `dt`-based
  rate.

---

## 21. Decisions & open forks

**Resolved** (recorded in `DECISIONS.md`):

1. **Version baseline (§2)** — **DL-010**: BBC disc / enhanced superset canonical,
   with a `cassette-pure` toggle as a later option.
2. **Yaw vs no-yaw (§8)** — **DL-011**: hybrid — faithful no-yaw default, optional
   yaw-assist toggle. (Reworks the axes shipped in Phase 1.)

**Still open** — raise and confirm before the dependent phase is built:

3. **Galactic-hyperdrive arrival scheme** — which system you land in per galaxy
   (fixed arrival vs conserved planet index). Minor; needed at Phase 3.
4. **Save format** — already open in DESIGN.md; the commander file stores cash,
   rank, FIST, equipment, galaxy/system, missions, kill count (default name
   "Jameson").
5. **Solid 3D vs wireframe** — *recommend solid 3D* preserving load-bearing
   proportions (§19).

---

## 22. Remake architecture mapping

How the spec maps onto our one-plugin-per-system layout (see `DESIGN.md`):

| Subsystem (this doc) | Plugin(s) | Phase |
|---|---|---|
| Flight, controls, camera, HUD/scanner | `flight`, `camera`, `hud` | 1 ✅ |
| Combat: weapons, shields, damage, missiles, AI, bounty | `combat` (+ submodules) | 2 |
| Galaxy generation, charts, hyperspace | `galaxy`/`universe` | 3 |
| Economy, commodities, contraband | `economy` | 4 |
| Stations, docking | `station` | 5 |
| Ranks, legal status, missions, save | `progression`, `missions` | 6 |

**Testing strategy:** golden-file tests for deterministic systems (all 2,048
generated systems; the QQ23 price/availability formula; rank & FIST thresholds);
behaviour tests (as in Phase 1) for flight/combat invariants.

---

## Sources

Primary: **bbcelite.com** (fully annotated 6502 disassembly of the BBC cassette,
disc, and ports), **wiki.alioth.net** (Elite wiki), the boxed **Space Trader's
Flight Training Manual**, **Ian Bell's official Elite pages**, and Wikipedia's
*Elite (1984 video game)*. Every load-bearing number above was cross-checked
against at least one primary source; items that could not be confirmed are marked
**(unverified)**. This document supersedes any uncited recollection of the game.
