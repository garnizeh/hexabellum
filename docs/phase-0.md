# Phase 0 Technical Spec — First Playable Vertical Slice

---

## Architectural Decision Record (ADR-001): Vertical Slice Strategy

### Context
When beginning a hybrid Rust + WebAssembly + TypeScript project, teams frequently fall into the "layered architecture trap": building the complete Rust game logic in isolation for months before writing a single line of WASM bindings or client rendering code. 

In browser gaming, this creates severe tail-risk:
1. WASM memory and serialization bottlenecks are discovered too late.
2. Rendering coordinates and game state models fall out of sync.
3. Turn loop synchronization and input latency issues require late-stage rewrites.
4. Motivation drops because developers cannot see or play with their progress.

### Decision
Hexabellum will be built through iterative, end-to-end **vertical slices**, starting with **Phase 0**.

| Technical Risk | Why a Vertical Slice Mitigates It |
|---|---|
| **Rust → WASM → TypeScript FFI Pipeline** | Proved immediately on Day 1 rather than at the end of the project. |
| **Grid Math vs. Canvas Rendering Sync** | Forces coordinate translation, hex tiling, and viewport scaling to be solved immediately. |
| **Turn Loop & State Serialization** | Validates the event-driven state architecture with real interactive feedback. |
| **Architectural Drift** | Every slice forces integration across crates and the frontend, preventing siloed code. |
| **Developer Velocity & Motivation** | Working interactive software is visible and testable within days. |

The primary technical unknown in Phase 0 is **not** game rules or AI — it is the **compilation, binding, and rendering pipeline**. Phase 0 exists to eliminate pipeline risk before complex mechanics are introduced.

---

## Phase 0 Goal

> **A running browser-based prototype where you can view a hex grid, select a hero, move to an adjacent hex, confirm the turn, and see the state update deterministically.**

Scope is strictly constrained: no combat, no action points (AP), no fog of war, and no networking. The focus is establishing an unbreakable foundation.

### Definition of Done (DoD)

- [ ] `hexabellum-core` compiles natively and passes all unit tests (`cargo test`).
- [ ] `wasm-pack` compiles `hexabellum-wasm` to standard ES module WASM without warnings.
- [ ] Vite dev server boots and loads the WASM module asynchronously in modern browsers.
- [ ] Pointy-topped hex grid renders with correct spacing, orientation, and colors.
- [ ] Two heroes are placed on the board (Player Team 0 on left, Team 1 on right).
- [ ] Clicking the player hero selects it, highlights adjacent walkable hexes, and updates the HUD.
- [ ] Clicking a highlighted target marks a planned movement with distinct visual feedback.
- [ ] Clicking "End Turn" passes orders to the engine, resolves the turn, increments the round counter, and animates/updates unit positions.
- [ ] This planning-resolution loop executes indefinitely without memory leaks or console errors.

---

## Architecture for Phase 0

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Client Browser Runtime                                │
│                                                                                 │
│   ┌──────────────────────────┐                   ┌───────────────────────────┐  │
│   │     TypeScript Client    │                   │   hexabellum-core (WASM)  │  │
│   │  ┌────────────────────┐  │  WASM FFI Calls   │  ┌─────────────────────┐  │  │
│   │  │ PixiJS Renderer    │  │                   │  │ GameEngine          │  │  │
│   │  │ - Pointy-top hexes │◄─┼───────────────────┼──┤ - State (Round/Map) │  │  │
│   │  │ - Units & HP bars  │  │   JSON Strings    │  │ - TurnOrders        │  │  │
│   │  └────────────────────┘  │  (State & Events) │  └──────────┬──────────┘  │  │
│   │  ┌────────────────────┐  │                   │             │             │  │
│   │  │ Input & HUD        │  │                   │  ┌──────────▼──────────┐  │  │
│   │  │ - Click hit-test   │──┼───────────────────┼─►│ TurnProcessor       │  │  │
│   │  │ - Order staging    │  │                   │  │ - Resolves movement │  │  │
│   │  └────────────────────┘  │                   │  └─────────────────────┘  │  │
│   └──────────────────────────┘                   └───────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

Phase 0 runs 100% locally in the browser runtime. The WASM boundary acts as the authoritative engine, mimicking the exact interface that a multiplayer server will expose in later phases.

---

## Project Structure

```
hexabellum/
├── Cargo.toml                  # Workspace definition
├── Makefile                    # Unified build & run automation
├── crates/
│   ├── core/                   # hexabellum-core: Pure Rust game logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          # Engine facade & public API
│   │       ├── hex.rs          # Axial/Cube math, distance, ring & spiral
│   │       ├── unit.rs         # Units, teams, and properties
│   │       ├── state.rs        # GameState, phases, and map queries
│   │       ├── turn.rs         # Order staging and TurnProcessor
│   │       └── event.rs        # GameEvent definitions (JSON-tagged)
│   │
│   └── wasm/                   # hexabellum-wasm: WebAssembly bindings
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs          # wasm-bindgen export layer (WasmGame)
│
└── web/                        # TypeScript + Vite + PixiJS client
    ├── package.json
    ├── tsconfig.json
    ├── vite.config.ts
    ├── index.html
    └── src/
        ├── main.ts             # Application bootstrapping
        ├── game/
        │   ├── bridge.ts       # WASM wrapper and typed JSON parser
        │   ├── renderer.ts     # PixiJS v8 pointy-top renderer
        │   └── input.ts        # Stage click hit-testing & order workflow
        └── wasm/
            └── pkg/            # Compiled wasm-pack output (git-ignored)
```

---

## Core Data Structures & Game Logic (Rust)

### 1. Hexagonal Grid Mathematics (`crates/core/src/hex.rs`)

Hexabellum uses **pointy-topped** hexagons using an **axial coordinate system** `(q, r)`. Axial coordinates are a 2D projection of 3D cube coordinates `(q, r, s)` constrained by the invariant:

$$q + r + s = 0 \iff s = -q - r$$

```
               -r (North)
                 ▲
      (0,-1)    │    (1,-1)
        NW \    │    / NE
            \   │   /
             \  │  /
   (-1,0) ──── (0,0) ──── (1,0)   +q (East)
     W       /  │  \       E
            /   │   \
        SW /    │    \ SE
     (-1,1)     │    (0,1)
                ▼
            +r (South)
```

#### Directional Vectors and Ring Generation
To traverse concentric rings around a hexagon without duplicate visits or coordinate drift, neighbor directions must be evaluated in cyclic counter-clockwise order:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Axial coordinates for the hexagonal grid.
/// q = column, r = row.
/// Cube constraint: q + r + s = 0 (where s = -q - r).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

/// The 6 axial unit directions in counter-clockwise cyclic order (pointy-top orientation).
pub const DIRECTIONS: [(i32, i32); 6] = [
    (1, 0),   // 0: East
    (0, 1),   // 1: South-East
    (-1, 1),  // 2: South-West
    (-1, 0),  // 3: West
    (0, -1),  // 4: North-West
    (1, -1),  // 5: North-East
];

impl HexCoord {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Third cube coordinate s, where q + r + s = 0.
    #[inline]
    pub const fn s(&self) -> i32 {
        -self.q - self.r
    }

    /// Cube distance between two coordinates: max(|Δq|, |Δr|, |Δs|).
    pub fn distance(&self, other: &HexCoord) -> u32 {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = (self.s() - other.s()).abs();
        ((dq + dr + ds) / 2) as u32
    }

    /// Get all 6 adjacent neighboring hexes.
    pub fn neighbors(&self) -> [HexCoord; 6] {
        [
            HexCoord::new(self.q + DIRECTIONS[0].0, self.r + DIRECTIONS[0].1),
            HexCoord::new(self.q + DIRECTIONS[1].0, self.r + DIRECTIONS[1].1),
            HexCoord::new(self.q + DIRECTIONS[2].0, self.r + DIRECTIONS[2].1),
            HexCoord::new(self.q + DIRECTIONS[3].0, self.r + DIRECTIONS[3].1),
            HexCoord::new(self.q + DIRECTIONS[4].0, self.r + DIRECTIONS[4].1),
            HexCoord::new(self.q + DIRECTIONS[5].0, self.r + DIRECTIONS[5].1),
        ]
    }

    /// Get all hexes on a ring at the given radius.
    /// Traverses the 6 sides cyclically, yielding exactly 6 * radius hexes.
    pub fn ring(&self, radius: u32) -> Vec<HexCoord> {
        if radius == 0 {
            return vec![*self];
        }

        let mut results = Vec::with_capacity((6 * radius) as usize);

        // Start at corner: self + DIRECTIONS[4] * radius (North-West)
        let mut current = HexCoord::new(
            self.q + (DIRECTIONS[4].0 * radius as i32),
            self.r + (DIRECTIONS[4].1 * radius as i32),
        );

        // Walk 6 sides, stepping `radius` times along each side direction
        for dir in DIRECTIONS {
            for _ in 0..radius {
                results.push(current);
                current = HexCoord::new(current.q + dir.0, current.r + dir.1);
            }
        }

        results
    }

    /// Get all hexes in a filled circle (spiral) up to `radius`.
    /// Total count equals 1 + 3 * radius * (radius + 1).
    pub fn spiral(&self, radius: u32) -> Vec<HexCoord> {
        let count = 1 + 3 * radius * (radius + 1);
        let mut results = Vec::with_capacity(count as usize);
        results.push(*self);
        for r in 1..=radius {
            results.extend(self.ring(r));
        }
        results
    }
}

/// The hex map definition containing radius and blocked tiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexMap {
    pub radius: u32,
    #[serde(default)]
    pub obstacles: HashSet<HexCoord>,
}

impl HexMap {
    pub fn new(radius: u32) -> Self {
        Self {
            radius,
            obstacles: HashSet::new(),
        }
    }

    /// Return all coordinates within the map bounds.
    pub fn all_hexes(&self) -> Vec<HexCoord> {
        let center = HexCoord::new(0, 0);
        center.spiral(self.radius)
    }

    /// True if coordinate is within radius bounds and not an obstacle.
    pub fn is_walkable(&self, coord: &HexCoord) -> bool {
        let center = HexCoord::new(0, 0);
        center.distance(coord) <= self.radius && !self.obstacles.contains(coord)
    }
}
```

---

### 2. Units & Entities (`crates/core/src/unit.rs`)

```rust
use crate::hex::HexCoord;
use serde::{Deserialize, Serialize};

pub type UnitId = u64;
pub type TeamId = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    Hero,
    Minion,
    Tower,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub team: TeamId,
    pub pos: HexCoord,
    pub hp: u32,
    pub max_hp: u32,
}

impl Unit {
    pub fn new_hero(id: UnitId, team: TeamId, pos: HexCoord) -> Self {
        Self {
            id,
            kind: UnitKind::Hero,
            team,
            pos,
            hp: 100,
            max_hp: 100,
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}
```

---

### 3. Game State (`crates/core/src/state.rs`)

```rust
use crate::hex::{HexCoord, HexMap};
use crate::unit::{TeamId, Unit, UnitId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Planning,
    Resolution,
    MatchEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub round: u32,
    pub phase: Phase,
    pub map: HexMap,
    pub units: HashMap<UnitId, Unit>,
    pub winner: Option<TeamId>,
}

impl GameState {
    pub fn new(map: HexMap) -> Self {
        Self {
            round: 0,
            phase: Phase::Planning,
            map,
            units: HashMap::new(),
            winner: None,
        }
    }

    pub fn add_unit(&mut self, unit: Unit) {
        self.units.insert(unit.id, unit);
    }

    pub fn get_unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(&id)
    }

    pub fn get_unit_at(&self, coord: &HexCoord) -> Option<&Unit> {
        self.units.values().find(|u| u.pos == *coord && u.is_alive())
    }

    pub fn is_occupied(&self, coord: &HexCoord) -> bool {
        self.get_unit_at(coord).is_some()
    }

    pub fn alive_units(&self) -> Vec<&Unit> {
        self.units.values().filter(|u| u.is_alive()).collect()
    }
}
```

---

### 4. Turn Resolution & Orders (`crates/core/src/turn.rs` & `event.rs`)

#### `crates/core/src/event.rs`
The JSON output uses `#[serde(tag = "type")]` so TypeScript receives clean tagged unions:
`{ "type": "UnitMoved", "unit_id": 1, "from": {"q": -3, "r": 0}, "to": {"q": -2, "r": 0} }`.

```rust
use crate::hex::HexCoord;
use crate::unit::UnitId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum GameEvent {
    RoundStarted { round: u32 },
    UnitMoved {
        unit_id: UnitId,
        from: HexCoord,
        to: HexCoord,
    },
    RoundEnded { round: u32 },
}
```

#### `crates/core/src/turn.rs`

```rust
use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::state::{GameState, Phase};
use crate::unit::UnitId;
use std::collections::HashMap;

/// Staged orders collected during the planning phase.
#[derive(Debug, Clone, Default)]
pub struct TurnOrders {
    pub move_orders: HashMap<UnitId, HexCoord>,
}

impl TurnOrders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_move(&mut self, unit_id: UnitId, target: HexCoord) {
        self.move_orders.insert(unit_id, target);
    }

    pub fn clear(&mut self) {
        self.move_orders.clear();
    }
}

pub struct TurnProcessor;

impl TurnProcessor {
    /// Resolve all staged orders and advance the game round.
    pub fn resolve(state: &mut GameState, orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        state.phase = Phase::Resolution;
        events.push(GameEvent::RoundStarted {
            round: state.round + 1,
        });

        // Resolve planned movements
        for (&unit_id, &target) in &orders.move_orders {
            if let Some(unit) = state.units.get_mut(&unit_id) {
                if !unit.is_alive() {
                    continue;
                }

                let from = unit.pos;

                // Validate: target must be walkable and unoccupied
                if !state.map.is_walkable(&target) {
                    continue;
                }
                if state.is_occupied(&target) && target != from {
                    continue;
                }

                unit.pos = target;
                events.push(GameEvent::UnitMoved {
                    unit_id,
                    from,
                    to: target,
                });
            }
        }

        // Advance round and return to planning
        state.round += 1;
        state.phase = Phase::Planning;

        events.push(GameEvent::RoundEnded { round: state.round });

        events
    }
}
```

---

### 5. Engine Facade (`crates/core/src/lib.rs`)

```rust
pub mod event;
pub mod hex;
pub mod state;
pub mod turn;
pub mod unit;

pub const PLAYER_TEAM: u8 = 0;
pub const ENEMY_TEAM: u8 = 1;

/// The authoritative game engine running inside WASM or native server.
pub struct GameEngine {
    state: state::GameState,
    pending_orders: turn::TurnOrders,
}

impl GameEngine {
    /// Initial 1v1 Phase 0 board: radius 4 (61 hexes), 3 central obstacles, 2 heroes.
    pub fn new() -> Self {
        let mut map = hex::HexMap::new(4);

        map.obstacles.insert(hex::HexCoord::new(0, 0));
        map.obstacles.insert(hex::HexCoord::new(1, -1));
        map.obstacles.insert(hex::HexCoord::new(-1, 1));

        let mut state = state::GameState::new(map);

        // Team 0 (Player) on left, Team 1 (Opponent) on right
        state.add_unit(unit::Unit::new_hero(1, PLAYER_TEAM, hex::HexCoord::new(-3, 0)));
        state.add_unit(unit::Unit::new_hero(2, ENEMY_TEAM, hex::HexCoord::new(3, 0)));

        Self {
            state,
            pending_orders: turn::TurnOrders::new(),
        }
    }

    pub fn state(&self) -> &state::GameState {
        &self.state
    }

    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap()
    }

    /// Plan a movement order. Rejects invalid units, dead units, enemy units, or non-adjacent tiles.
    pub fn set_move_order(&mut self, unit_id: unit::UnitId, q: i32, r: i32) -> bool {
        let target = hex::HexCoord::new(q, r);

        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive()
                && unit.team == PLAYER_TEAM
                && unit.pos.distance(&target) == 1
                && self.state.map.is_walkable(&target)
                && !self.state.is_occupied(&target)
            {
                self.pending_orders.set_move(unit_id, target);
                return true;
            }
        }
        false
    }

    /// Execute turn resolution and return emitted GameEvents.
    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::take(&mut self.pending_orders);
        let events = turn::TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    pub fn get_map_hexes(&self) -> String {
        let hexes: Vec<(i32, i32)> = self
            .state
            .map
            .all_hexes()
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&hexes).unwrap()
    }

    pub fn get_obstacles(&self) -> String {
        let obstacles: Vec<(i32, i32)> = self
            .state
            .map
            .obstacles
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&obstacles).unwrap()
    }

    /// Query walkable, unoccupied adjacent hexes for a given unit.
    pub fn get_move_targets(&self, unit_id: unit::UnitId) -> String {
        let targets: Vec<(i32, i32)> = if let Some(unit) = self.state.get_unit(unit_id) {
            unit.pos
                .neighbors()
                .iter()
                .filter(|h| self.state.map.is_walkable(h) && !self.state.is_occupied(h))
                .map(|h| (h.q, h.r))
                .collect()
        } else {
            vec![]
        };
        serde_json::to_string(&targets).unwrap()
    }
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## WASM Bindings (`crates/wasm`)

### `crates/wasm/src/lib.rs`

```rust
use hexabellum_core::GameEngine;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmGame {
    engine: GameEngine,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmGame {
        WasmGame {
            engine: GameEngine::new(),
        }
    }

    pub fn get_state(&self) -> String {
        self.engine.get_state()
    }

    pub fn set_move_order(&mut self, unit_id: u64, q: i32, r: i32) -> bool {
        self.engine.set_move_order(unit_id, q, r)
    }

    pub fn end_turn(&mut self) -> String {
        self.engine.end_turn()
    }

    pub fn get_map_hexes(&self) -> String {
        self.engine.get_map_hexes()
    }

    pub fn get_obstacles(&self) -> String {
        self.engine.get_obstacles()
    }

    pub fn get_move_targets(&self, unit_id: u64) -> String {
        self.engine.get_move_targets(unit_id)
    }
}

impl Default for WasmGame {
    fn default() -> Self {
        Self::new()
    }
}
```

### Cargo Manifests

#### `Cargo.toml` (Workspace Root)
```toml
[workspace]
members = [
    "crates/core",
    "crates/wasm",
]
resolver = "2"
```

#### `crates/core/Cargo.toml`
```toml
[package]
name = "hexabellum-core"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

#### `crates/wasm/Cargo.toml`
```toml
[package]
name = "hexabellum-wasm"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
hexabellum-core = { path = "../core" }
wasm-bindgen = "0.2"
serde_json = "1.0"
```

---

## TypeScript Client & PixiJS Rendering (`web/`)

### `web/package.json`

```json
{
  "name": "hexabellum-web",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "build-wasm": "cd ../crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg"
  },
  "dependencies": {
    "pixi.js": "^8.0.0"
  },
  "devDependencies": {
    "typescript": "^5.4.0",
    "vite": "^5.2.0"
  }
}
```

### `web/vite.config.ts`

```typescript
import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 5173,
    fs: {
      allow: ['..'],
    },
  },
});
```

### `web/tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"]
  },
  "include": ["src"]
}
```

---

### WASM Bridge (`web/src/game/bridge.ts`)

```typescript
import init, { WasmGame } from '../wasm/pkg/hexabellum_wasm';

let game: WasmGame | null = null;

export interface HexCoord {
  q: number;
  r: number;
}

export interface UnitData {
  id: number;
  kind: 'Hero' | 'Minion' | 'Tower' | 'Neutral';
  team: number;
  pos: HexCoord;
  hp: number;
  max_hp: number;
}

export interface GameState {
  round: number;
  phase: 'Planning' | 'Resolution' | 'MatchEnd';
  units: Record<string, UnitData>;
  winner: number | null;
}

export type GameEvent =
  | { type: 'RoundStarted'; round: number }
  | { type: 'UnitMoved'; unit_id: number; from: HexCoord; to: HexCoord }
  | { type: 'RoundEnded'; round: number };

export async function initGame(): Promise<void> {
  await init();
  game = new WasmGame();
}

export function getState(): GameState {
  if (!game) throw new Error('Game engine not initialized');
  return JSON.parse(game.get_state());
}

export function setMoveOrder(unitId: number, q: number, r: number): boolean {
  if (!game) throw new Error('Game engine not initialized');
  return game.set_move_order(unitId, q, r);
}

export function endTurn(): GameEvent[] {
  if (!game) throw new Error('Game engine not initialized');
  return JSON.parse(game.end_turn());
}

export function getMapHexes(): HexCoord[] {
  if (!game) throw new Error('Game engine not initialized');
  return JSON.parse(game.get_map_hexes()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getObstacles(): HexCoord[] {
  if (!game) throw new Error('Game engine not initialized');
  return JSON.parse(game.get_obstacles()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getMoveTargets(unitId: number): HexCoord[] {
  if (!game) throw new Error('Game engine not initialized');
  return JSON.parse(game.get_move_targets(unitId)).map(([q, r]: [number, number]) => ({ q, r }));
}
```

---

### PixiJS Renderer (`web/src/game/renderer.ts`)

#### Pointy-Topped Alignment Details
For pointy-topped hexagons with radius $R = \text{HEX\_SIZE}$:
- Center $X = R \cdot (\sqrt{3} q + \frac{\sqrt{3}}{2} r) + \text{centerX}$
- Center $Y = R \cdot (\frac{3}{2} r) + \text{centerY}$
- Vertex angles: $\theta_i = \frac{\pi}{3} i - \frac{\pi}{6}$ (angles at $30^\circ, 90^\circ, 150^\circ, 210^\circ, 270^\circ, 330^\circ$)

```typescript
import * as PIXI from 'pixi.js';
import { HexCoord, GameState } from './bridge';

export const HEX_SIZE = 30; // Radius in pixels

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer: PIXI.Container;
  private overlayLayer: PIXI.Container;
  private unitLayer: PIXI.Container;

  constructor(canvas: HTMLCanvasElement) {
    this.app = new PIXI.Application();

    // Async init for PixiJS v8
    this.app.init({
      canvas,
      resizeTo: window,
      backgroundColor: 0x1a1a2e,
      antialias: true,
      autoDensity: true,
      resolution: window.devicePixelRatio || 1,
    }).then(() => {
      this.app.stage.eventMode = 'static';
      this.app.stage.hitArea = this.app.screen;
    });

    this.hexLayer = new PIXI.Container();
    this.overlayLayer = new PIXI.Container();
    this.unitLayer = new PIXI.Container();

    this.app.stage.addChild(this.hexLayer);
    this.app.stage.addChild(this.overlayLayer);
    this.app.stage.addChild(this.unitLayer);
  }

  /** Convert axial hex coords to screen pixel position (pointy-topped). */
  hexToPixel(q: number, r: number): { x: number; y: number } {
    const x = HEX_SIZE * (Math.sqrt(3) * q + (Math.sqrt(3) / 2) * r);
    const y = HEX_SIZE * (1.5 * r);
    return {
      x: x + this.app.screen.width / 2,
      y: y + this.app.screen.height / 2,
    };
  }

  /** Render the base hex tiles and obstacles. */
  drawMap(hexes: HexCoord[], obstacles: HexCoord[]): void {
    this.hexLayer.removeChildren();
    const obstacleSet = new Set(obstacles.map(h => `${h.q},${h.r}`));

    for (const hex of hexes) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);
      const isObstacle = obstacleSet.has(`${hex.q},${hex.r}`);

      const g = new PIXI.Graphics();

      // Pointy-top hex vertex points: angles (i * 60° - 30°)
      const points: number[] = [];
      for (let i = 0; i < 6; i++) {
        const angle = (Math.PI / 3) * i - Math.PI / 6;
        points.push(x + HEX_SIZE * Math.cos(angle), y + HEX_SIZE * Math.sin(angle));
      }

      g.poly(points);
      if (isObstacle) {
        g.fill({ color: 0x2d2d44 });
        g.stroke({ color: 0x444466, width: 1 });
      } else {
        g.fill({ color: 0x16213e });
        g.stroke({ color: 0x0f3460, width: 1 });
      }

      this.hexLayer.addChild(g);
    }
  }

  /** Render units, team colors, selection indicator, and HP bars. */
  drawUnits(state: GameState, selectedUnitId: number | null = null): void {
    this.unitLayer.removeChildren();

    for (const [idStr, unit] of Object.entries(state.units)) {
      const id = Number(idStr);
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

      const g = new PIXI.Graphics();
      const isSelected = selectedUnitId === id;
      const teamColor = unit.team === 0 ? 0x4fc3f7 : 0xef5350;

      // Selection ring
      if (isSelected) {
        g.circle(x, y, HEX_SIZE * 0.7);
        g.stroke({ color: 0xffeb3b, width: 2 });
      }

      // Unit token
      g.circle(x, y, HEX_SIZE * 0.5);
      g.fill({ color: teamColor });
      g.stroke({ color: 0xffffff, width: 2 });

      // Health bar
      const hpWidth = HEX_SIZE * 0.8;
      const hpHeight = 4;
      const hpX = x - hpWidth / 2;
      const hpY = y - HEX_SIZE * 0.75;
      const hpRatio = Math.max(0, Math.min(1, unit.hp / unit.max_hp));

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x333333 });
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x4caf50 : 0xff9800 });

      this.unitLayer.addChild(g);
    }
  }

  /** Highlight available move destinations and staged target. */
  drawMoveTargets(targets: HexCoord[], plannedTarget: HexCoord | null = null): void {
    this.overlayLayer.removeChildren();

    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);
      const isPlanned = plannedTarget && plannedTarget.q === target.q && plannedTarget.r === target.r;

      const g = new PIXI.Graphics();
      if (isPlanned) {
        // Distinct amber indicator for staged move
        g.circle(x, y, HEX_SIZE * 0.4);
        g.fill({ color: 0xffeb3b, alpha: 0.7 });
        g.stroke({ color: 0xffffff, width: 2 });
      } else {
        // Green highlight for available adjacent steps
        g.circle(x, y, HEX_SIZE * 0.3);
        g.fill({ color: 0x4caf50, alpha: 0.5 });
      }
      this.overlayLayer.addChild(g);
    }
  }

  clearMoveTargets(): void {
    this.overlayLayer.removeChildren();
  }

  getApp(): PIXI.Application {
    return this.app;
  }

  getStage(): PIXI.Container {
    return this.app.stage;
  }
}
```

---

### Input Handler (`web/src/game/input.ts`)

```typescript
import * as PIXI from 'pixi.js';
import { HexRenderer, HEX_SIZE } from './renderer';
import { getState, setMoveOrder, getMoveTargets, endTurn, HexCoord } from './bridge';

export class InputHandler {
  private selectedUnit: number | null = null;
  private plannedMove: HexCoord | null = null;
  private renderer: HexRenderer;
  private onStateChange: (() => void) | null = null;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  setOnStateChange(cb: () => void): void {
    this.onStateChange = cb;
  }

  private setupListeners(): void {
    const app = this.renderer.getApp();
    const stage = this.renderer.getStage();

    stage.eventMode = 'static';
    stage.hitArea = app.screen;

    stage.on('pointerdown', (event: PIXI.FederatedPointerEvent) => {
      this.handleClick(event.global.x, event.global.y);
    });
  }

  private handleClick(pixelX: number, pixelY: number): void {
    const state = getState();
    const hex = this.pixelToHex(pixelX, pixelY);
    if (!hex) return;

    // Check if player clicked a unit
    for (const [idStr, unit] of Object.entries(state.units)) {
      const id = Number(idStr);
      if (unit.pos.q === hex.q && unit.pos.r === hex.r) {
        if (unit.team === 0) {
          // Toggle selection off if already selected
          if (this.selectedUnit === id) {
            this.deselect();
            return;
          }

          this.selectedUnit = id;
          this.plannedMove = null;
          const targets = getMoveTargets(id);
          this.renderer.drawMoveTargets(targets);
          this.renderer.drawUnits(state, this.selectedUnit);
          this.onStateChange?.();
          return;
        }
      }
    }

    // If unit is already selected, attempt to plan move
    if (this.selectedUnit !== null) {
      const targets = getMoveTargets(this.selectedUnit);
      const isTargetValid = targets.some(t => t.q === hex.q && t.r === hex.r);

      if (isTargetValid) {
        const success = setMoveOrder(this.selectedUnit, hex.q, hex.r);
        if (success) {
          this.plannedMove = hex;
          this.renderer.drawMoveTargets(targets, this.plannedMove);
          this.onStateChange?.();
          return;
        }
      }

      // Clicking outside valid targets deselects
      this.deselect();
    }
  }

  private deselect(): void {
    this.selectedUnit = null;
    this.plannedMove = null;
    this.renderer.clearMoveTargets();
    this.renderer.drawUnits(getState(), null);
    this.onStateChange?.();
  }

  /** Inverse projection: screen pixels to nearest axial hex coordinate. */
  private pixelToHex(x: number, y: number): HexCoord | null {
    const app = this.renderer.getApp();
    const cx = x - app.screen.width / 2;
    const cy = y - app.screen.height / 2;

    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / HEX_SIZE;
    const r = ((2 / 3) * cy) / HEX_SIZE;

    return this.hexRound(q, r);
  }

  /** Fractional axial rounding via cube coordinate distance clamping. */
  private hexRound(q: number, r: number): HexCoord {
    const s = -q - r;
    let rq = Math.round(q);
    let rr = Math.round(r);
    const rs = Math.round(s);

    const qDiff = Math.abs(rq - q);
    const rDiff = Math.abs(rr - r);
    const sDiff = Math.abs(rs - s);

    if (qDiff > rDiff && qDiff > sDiff) {
      rq = -rr - rs;
    } else if (rDiff > sDiff) {
      rr = -rq - rs;
    }

    return { q: rq, r: rr };
  }

  endTurn(): void {
    const events = endTurn();
    this.selectedUnit = null;
    this.plannedMove = null;
    this.renderer.clearMoveTargets();

    const state = getState();
    this.renderer.drawUnits(state, null);
    this.onStateChange?.();

    console.log('Turn resolved events:', events);
  }

  getSelectedUnit(): number | null {
    return this.selectedUnit;
  }

  getPlannedMove(): HexCoord | null {
    return this.plannedMove;
  }
}
```

---

### Main Entry Point (`web/src/main.ts`)

```typescript
import { initGame, getState, getMapHexes, getObstacles } from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';

async function main() {
  await initGame();

  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  const renderer = new HexRenderer(canvas);

  const hexes = getMapHexes();
  const obstacles = getObstacles();
  const state = getState();

  renderer.drawMap(hexes, obstacles);
  renderer.drawUnits(state);

  const input = new InputHandler(renderer);

  const endTurnBtn = document.getElementById('end-turn') as HTMLButtonElement;
  const roundDisplay = document.getElementById('round') as HTMLElement;
  const selectionDisplay = document.getElementById('selection') as HTMLElement;

  const updateHud = () => {
    const currentState = getState();
    roundDisplay.textContent = `Round ${currentState.round}`;
    const sel = input.getSelectedUnit();
    const planned = input.getPlannedMove();
    if (sel !== null) {
      selectionDisplay.textContent = planned
        ? `Hero #${sel} → Move staged to (${planned.q}, ${planned.r})`
        : `Hero #${sel} selected (Click an adjacent hex to stage move)`;
    } else {
      selectionDisplay.textContent = 'Select your blue hero';
    }
  };

  input.setOnStateChange(updateHud);

  endTurnBtn.addEventListener('click', () => {
    input.endTurn();
  });

  updateHud();

  console.log('Hexabellum Phase 0 active.');
}

main().catch(console.error);
```

---

### HTML Document (`web/index.html`)

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Hexabellum - Phase 0 Vertical Slice</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      background: #0a0a1a;
      color: #eee;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      overflow: hidden;
      user-select: none;
    }
    #game-canvas {
      display: block;
      width: 100vw;
      height: 100vh;
    }
    #hud {
      position: fixed;
      top: 16px;
      left: 16px;
      z-index: 10;
      display: flex;
      flex-direction: column;
      gap: 8px;
      background: rgba(10, 10, 26, 0.85);
      padding: 14px 18px;
      border-radius: 8px;
      border: 1px solid #1a2744;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
    }
    #round {
      font-size: 20px;
      font-weight: 700;
      color: #4fc3f7;
      letter-spacing: 0.5px;
    }
    #selection {
      font-size: 13px;
      color: #9fd8ff;
      min-height: 18px;
    }
    #end-turn {
      margin-top: 4px;
      padding: 10px 20px;
      background: #0f3460;
      color: #4fc3f7;
      border: 1px solid #4fc3f7;
      border-radius: 6px;
      font-size: 14px;
      font-weight: 600;
      cursor: pointer;
      transition: background 0.15s ease, transform 0.05s ease;
    }
    #end-turn:hover {
      background: #194880;
    }
    #end-turn:active {
      transform: scale(0.98);
    }
    #instructions {
      position: fixed;
      bottom: 16px;
      left: 16px;
      font-size: 13px;
      color: #78909c;
      background: rgba(10, 10, 26, 0.85);
      padding: 8px 14px;
      border-radius: 6px;
      border: 1px solid #1a2744;
    }
  </style>
</head>
<body>
  <div id="hud">
    <div id="round">Round 0</div>
    <div id="selection">Select your blue hero</div>
    <button id="end-turn">End Turn</button>
  </div>
  <canvas id="game-canvas"></canvas>
  <div id="instructions">
    Click Blue Hero → Click Green Target to Plan Move → Click End Turn
  </div>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

---

## Build Automation (`Makefile`)

```makefile
.PHONY: all build-wasm dev build test clean

# Default: compile wasm and start dev server
all: dev

# Run Rust core tests
test:
	cargo test --workspace

# Build WebAssembly package via wasm-pack
build-wasm:
	cd crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg

# Start Vite development server
dev: build-wasm
	cd web && npm run dev

# Production build
build: build-wasm
	cd web && npm run build

# Clean build artifacts
clean:
	rm -rf web/src/wasm/pkg
	rm -rf web/dist
	cargo clean
```

---

## Automated Unit Testing Suite (`crates/core`)

Add the following unit test suite to `crates/core/src/lib.rs` (or `crates/core/tests/phase0_tests.rs`) to verify all core invariants before browser integration:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::HexCoord;

    #[test]
    fn test_hex_distance_and_symmetry() {
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(2, -1);
        assert_eq!(a.distance(&b), 2);
        assert_eq!(b.distance(&a), 2);
        assert_eq!(a.distance(&a), 0);
    }

    #[test]
    fn test_ring_uniqueness_and_counts() {
        let center = HexCoord::new(0, 0);
        for radius in 1..=4 {
            let ring = center.ring(radius);
            assert_eq!(ring.len(), (6 * radius) as usize);

            // Verify no duplicates
            let unique: std::collections::HashSet<_> = ring.iter().copied().collect();
            assert_eq!(unique.len(), (6 * radius) as usize);

            // Verify all points are at exact distance
            for h in &ring {
                assert_eq!(center.distance(h), radius);
            }
        }
    }

    #[test]
    fn test_spiral_count_exact() {
        let center = HexCoord::new(0, 0);
        // Radius 4: 1 + 3 * 4 * 5 = 61 hexes
        let spiral = center.spiral(4);
        assert_eq!(spiral.len(), 61);

        let unique: std::collections::HashSet<_> = spiral.iter().copied().collect();
        assert_eq!(unique.len(), 61);
    }

    #[test]
    fn test_walkable_bounds_and_obstacles() {
        let mut map = hex::HexMap::new(2);
        let center = HexCoord::new(0, 0);
        let obstacle = HexCoord::new(1, 0);
        map.obstacles.insert(obstacle);

        assert!(map.is_walkable(&center));
        assert!(!map.is_walkable(&obstacle));
        assert!(!map.is_walkable(&HexCoord::new(3, 0))); // Out of bounds
    }

    #[test]
    fn test_move_order_staging_and_resolution() {
        let mut engine = GameEngine::new();
        let hero_id = 1; // Team 0 hero at (-3, 0)

        // Valid move to adjacent walkable tile
        assert!(engine.set_move_order(hero_id, -2, 0));

        let events_json = engine.end_turn();
        assert!(events_json.contains("RoundStarted"));
        assert!(events_json.contains("UnitMoved"));
        assert!(events_json.contains("RoundEnded"));

        let hero = engine.state().get_unit(hero_id).unwrap();
        assert_eq!(hero.pos, HexCoord::new(-2, 0));
        assert_eq!(engine.state().round, 1);
    }

    #[test]
    fn test_move_order_rejection() {
        let mut engine = GameEngine::new();
        // Disallow moving non-adjacent tiles in Phase 0
        assert!(!engine.set_move_order(1, 0, 0));
        // Disallow ordering enemy units
        assert!(!engine.set_move_order(2, 2, 0));
    }
}
```

---

## Verification & Acceptance Criteria

| # | Acceptance Criterion | Test Verification Method | Status |
|---|---|---|:---:|
| 1 | `cargo test --workspace` passes cleanly | Automated test suite execution | [x] |
| 2 | `wasm-pack build --target web` succeeds | Terminal output produces `web/src/wasm/pkg` | [x] |
| 3 | Vite dev server serves page with zero console errors | Chrome DevTools console clean | [x] |
| 4 | Pointy-topped hex grid renders seamlessly without seams or distortion | Visual check of canvas borders | [x] |
| 5 | Three dark obstacles visible in center tiles | Visual check: `(0,0), (1,-1), (-1,1)` | [x] |
| 6 | Two heroes present: Blue at `(-3, 0)`, Red at `(3, 0)` | Visual token rendering & HP bars | [x] |
| 7 | Clicking Blue Hero highlights adjacent tiles in green and shows selection ring | Interactive click test | [x] |
| 8 | Clicking adjacent tile turns target amber and updates HUD with staged target | Interactive click test | [x] |
| 9 | Clicking "End Turn" moves the hero, clears overlay, and increments round | Engine state & HUD round counter | [x] |
| 10 | Loop repeats across multiple rounds deterministically | Manual 5-round play session | [x] |

---

## Troubleshooting & Common Pitfalls

### 1. `wasm-pack` Output Directory
- **Problem**: `wasm-pack` creates artifacts in `crates/wasm/pkg` if `--out-dir` is omitted.
- **Fix**: Use `--out-dir ../../web/src/wasm/pkg` so Vite can import directly from `web/src/wasm/pkg`.

### 2. Vite MIME Type Error (`application/wasm`)
- **Problem**: Browser fails with `Failed to load module script: Expected a JavaScript module script but the server responded with a MIME type of "application/wasm"`.
- **Fix**: In `web/src/game/bridge.ts`, ensure `import init from '../wasm/pkg/hexabellum_wasm'` is used and `await init()` is executed before constructing `new WasmGame()`.

### 3. Hexagon Overlap or Gaps
- **Problem**: Hexagons appear clipped, diamond-shaped, or rotated.
- **Cause**: Using flat-top vertex angles ($0^\circ, 60^\circ, 120^\circ$) with pointy-top axial center coordinates ($x = \sqrt{3} q + \frac{\sqrt{3}}{2} r$).
- **Fix**: Pointy-top hex vertices require rotating by $-30^\circ$: `(Math.PI / 3) * i - Math.PI / 6`.

### 4. High-DPI Display Blurriness
- **Fix**: Set `resolution: window.devicePixelRatio || 1` and `autoDensity: true` in `app.init(...)`.

---

## What is NOT in Phase 0 (Phase 1+ Scope)

To preserve velocity and avoid scope creep, the following systems are deferred to **Phase 1**:

- ❌ **Pathfinding**: Phase 0 only permits single-hex adjacent steps. (A* arrives in Phase 1).
- ❌ **Action Point (AP) System**: Unlimited 1 step per turn in Phase 0.
- ❌ **Initiative Queue**: Direct order processing in Phase 0.
- ❌ **Combat / Damage / Death**: Heroes cannot attack or take damage yet.
- ❌ **Enemy AI**: Team 1 hero stands idle as a test dummy.
- ❌ **Fog of War & Vision**: Full map visibility.
- ❌ **Networking & WebSockets**: Single-player local WASM execution.

---

## Evolution to Phase 1

Once Phase 0 is verified, Phase 1 expands the slice into a competitive tactical battle:

```
Phase 0 (Foundation)            Phase 1 (Tactical Combat)
─────────────────────            ─────────────────────────
1v1 heroes                  ──►  3v3 hero teams
Direct 1-hex step           ──►  A* multi-hex pathfinding
No resource budget          ──►  Action Point (AP) economy
No combat                   ──►  Attack orders, damage, and death
Passive opponent            ──►  Authoritative Enemy AI
Indefinite turns            ──►  Win/Loss condition & round timer
```
