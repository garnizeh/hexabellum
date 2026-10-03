# Phase 0 Technical Spec — First Playable Vertical Slice

## Do I agree with vertical slices?

**Yes, absolutely.** For this project, vertical slices are the right call. Here's why:

| Risk | Why vertical slice helps |
|------|--------------------------|
| Rust → WASM → TS integration | Proved immediately, not at the end |
| Rendering + game state sync | Forces you to solve it on day 1 |
| Turn loop architecture | Validated with real interaction |
| Motivation | You see something moving in days, not weeks |
| Architecture drift | Each slice forces integration, preventing siloed code |

The biggest technical risk in this project is **not** the game logic — it's the **pipeline**: Rust core compiling to WASM, running in the browser, communicating with a TypeScript renderer. A vertical slice kills that risk immediately.

---

## Phase 0 Goal

> **A running browser-based prototype where you can see a hex grid, select a hero, move it, end the turn, and see the updated state.**

That's it. No combat. No AI. No AP. No fog. Just the pipeline working end-to-end.

### Definition of Done

- [ ] Rust core compiles to WASM
- [ ] WASM module loads in browser
- [ ] Hex grid renders on screen (PixiJS or Canvas)
- [ ] One hero per team is placed on the grid
- [ ] Player can click a hero to select it
- [ ] Player can click a hex to set a move target
- [ ] Player can confirm/end turn
- [ ] Engine processes the movement
- [ ] Board updates to show new position
- [ ] Round counter increments
- [ ] This loop repeats indefinitely

---

## Architecture for Phase 0

```
┌─────────────────────────────────────────────────┐
│                  Browser                         │
│                                                  │
│  ┌──────────────┐       ┌────────────────────┐  │
│  │  TypeScript   │◄─────►│  moba-core (WASM)  │  │
│  │  UI + Render  │       │  Game State + Rules │  │
│  │  (PixiJS)     │       │  Turn Loop          │  │
│  └──────────────┘       └────────────────────┘  │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │  Input Handler                            │   │
│  │  (click → select / move / end turn)       │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

No server yet. No networking. The engine runs locally in WASM.

---

## Project Structure

```
hexabellum/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── core/                   # moba-core (pure logic)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── hex.rs          # hex math
│   │       ├── state.rs        # game state
│   │       ├── unit.rs         # unit types
│   │       ├── turn.rs         # turn loop
│   │       └── event.rs        # game events
│   │
│   └── wasm/                   # moba-wasm (bindings)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs          # wasm-bindgen exports
│
├── web/                        # TypeScript client
│   ├── package.json
│   ├── index.html
│   ├── vite.config.ts
│   └── src/
│       ├── main.ts             # entry point
│       ├── game/
│       │   ├── renderer.ts     # PixiJS rendering
│       │   ├── input.ts        # click handling
│       │   └── bridge.ts       # WASM API wrapper
│       └── ui/
│           ├── hud.ts          # round counter, selected unit
│           └── hex-overlay.ts  # movement highlights
│
└── Makefile                    # build scripts
```

---

## Core Data Structures (Rust)

### `crates/core/src/hex.rs`

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashSet;

/// Axial coordinates for hex grid.
/// q = column, r = row
/// Cube constraint: q + r + s = 0 (s is implicit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Distance between two hexes (cube distance).
    pub fn distance(&self, other: &HexCoord) -> u32 {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = ((-self.q - self.r) - (-other.q - other.r)).abs();
        ((dq + dr + ds) / 2) as u32
    }

    /// Get all 6 neighboring hexes.
    pub fn neighbors(&self) -> [HexCoord; 6] {
        [
            HexCoord::new(self.q + 1, self.r),
            HexCoord::new(self.q - 1, self.r),
            HexCoord::new(self.q, self.r + 1),
            HexCoord::new(self.q, self.r - 1),
            HexCoord::new(self.q + 1, self.r - 1),
            HexCoord::new(self.q - 1, self.r + 1),
        ]
    }

    /// Get all hexes within a given radius.
    pub fn ring(&self, radius: u32) -> Vec<HexCoord> {
        if radius == 0 {
            return vec![*self];
        }
        let mut results = Vec::new();
        // Start from one direction and walk around
        let directions = [
            (1, 0), (-1, 0), (0, 1), (0, -1), (1, -1), (-1, 1)
        ];
        // Walk to starting position
        let mut current = HexCoord::new(
            self.q + (directions[4].0 * radius as i32),
            self.r + (directions[4].1 * radius as i32),
        );
        for i in 0..6 {
            for _ in 0..radius {
                results.push(current);
                current = HexCoord::new(
                    current.q + directions[i].0,
                    current.r + directions[i].1,
                );
            }
        }
        results
    }

    /// Get all hexes in a filled radius (for map generation).
    pub fn spiral(&self, radius: u32) -> Vec<HexCoord> {
        let mut results = vec![*self];
        for r in 1..=radius {
            results.extend(self.ring(r));
        }
        results
    }
}

/// The hex map definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexMap {
    pub radius: u32,
    pub obstacles: HashSet<HexCoord>,
}

impl HexMap {
    pub fn new(radius: u32) -> Self {
        Self {
            radius,
            obstacles: HashSet::new(),
        }
    }

    /// Get all walkable hexes.
    pub fn all_hexes(&self) -> Vec<HexCoord> {
        let center = HexCoord::new(0, 0);
        center.spiral(self.radius)
    }

    /// Check if a hex is walkable.
    pub fn is_walkable(&self, coord: &HexCoord) -> bool {
        let center = HexCoord::new(0, 0);
        center.distance(coord) <= self.radius && !self.obstacles.contains(coord)
    }
}
```

### `crates/core/src/unit.rs`

```rust
use crate::hex::HexCoord;
use serde::{Serialize, Deserialize};

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
    // Future: ap, initiative, vision_range, etc.
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

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}
```

### `crates/core/src/state.rs`

```rust
use crate::hex::{HexCoord, HexMap};
use crate::unit::{Unit, UnitId, TeamId};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

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

    /// Check if a hex is occupied by any alive unit.
    pub fn is_occupied(&self, coord: &HexCoord) -> bool {
        self.get_unit_at(coord).is_some()
    }

    /// Get all alive units.
    pub fn alive_units(&self) -> Vec<&Unit> {
        self.units.values().filter(|u| u.is_alive()).collect()
    }

    /// Get all alive units for a team.
    pub fn team_units(&self, team: TeamId) -> Vec<&Unit> {
        self.units.values()
            .filter(|u| u.team == team && u.is_alive())
            .collect()
    }
}
```

### `crates/core/src/event.rs`

```rust
use crate::hex::HexCoord;
use crate::unit::UnitId;
use serde::{Serialize, Deserialize};

/// Events emitted during resolution.
/// The client uses these to animate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    RoundStarted { round: u32 },
    UnitMoved { unit_id: UnitId, from: HexCoord, to: HexCoord },
    RoundEnded { round: u32 },
    // Future: UnitAttacked, UnitDied, etc.
}
```

### `crates/core/src/turn.rs`

```rust
use crate::state::{GameState, Phase};
use crate::event::GameEvent;
use crate::hex::HexCoord;
use crate::unit::UnitId;
use std::collections::HashMap;

/// Player orders collected during planning phase.
#[derive(Debug, Clone, Default)]
pub struct TurnOrders {
    /// unit_id -> target hex for movement
    pub move_orders: HashMap<UnitId, HexCoord>,
}

impl TurnOrders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_move(&mut self, unit_id: UnitId, target: HexCoord) {
        self.move_orders.insert(unit_id, target);
    }
}

/// The turn processor.
/// Phase 0: simple movement resolution.
pub struct TurnProcessor;

impl TurnProcessor {
    /// Resolve the current round.
    /// Returns events for the client to animate.
    pub fn resolve(state: &mut GameState, orders: &TurnOrders) -> Vec<GameEvent> {
        let mut events = Vec::new();

        events.push(GameEvent::RoundStarted { round: state.round + 1 });

        // Phase 0: simple movement, no initiative, no AP
        for (unit_id, target) in &orders.move_orders {
            if let Some(unit) = state.units.get_mut(unit_id) {
                if !unit.is_alive() {
                    continue;
                }

                let from = unit.pos;

                // Validate: target must be walkable and not occupied
                if !state.map.is_walkable(target) {
                    continue;
                }
                if state.is_occupied(target) && *target != from {
                    continue;
                }

                // Move the unit
                unit.pos = *target;

                events.push(GameEvent::UnitMoved {
                    unit_id: *unit_id,
                    from,
                    to: *target,
                });
            }
        }

        // Advance round
        state.round += 1;
        state.phase = Phase::Planning;

        events.push(GameEvent::RoundEnded { round: state.round });

        events
    }
}
```

### `crates/core/src/lib.rs`

```rust
pub mod hex;
pub mod unit;
pub mod state;
pub mod turn;
pub mod event;

/// Top-level game engine.
/// This is the main entry point for WASM.
pub struct GameEngine {
    state: state::GameState,
    pending_orders: turn::TurnOrders,
}

impl GameEngine {
    /// Create a new game with default Phase 0 setup.
    pub fn new() -> Self {
        let mut map = hex::HexMap::new(4); // radius 4 = 61 hexes

        // Add a few obstacles for visual interest
        map.obstacles.insert(hex::HexCoord::new(0, 0));
        map.obstacles.insert(hex::HexCoord::new(1, -1));
        map.obstacles.insert(hex::HexCoord::new(-1, 1));

        let mut state = state::GameState::new(map);

        // Place heroes: team 0 on left, team 1 on right
        state.add_unit(unit::Unit::new_hero(
            1, 0, hex::HexCoord::new(-3, 0),
        ));
        state.add_unit(unit::Unit::new_hero(
            2, 1, hex::HexCoord::new(3, 0),
        ));

        Self {
            state,
            pending_orders: turn::TurnOrders::new(),
        }
    }

    /// Get current game state (serialized for client).
    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap()
    }

    /// Select a unit and set move target.
    pub fn set_move_order(&mut self, unit_id: unit::UnitId, q: i32, r: i32) -> bool {
        let target = hex::HexCoord::new(q, r);

        // Validate: unit exists, is alive, belongs to team 0 (player team for now)
        if let Some(unit) = self.state.get_unit(unit_id) {
            if unit.is_alive() && unit.team == 0 {
                self.pending_orders.set_move(unit_id, target);
                return true;
            }
        }
        false
    }

    /// End the turn and resolve.
    /// Returns events as JSON for the client.
    pub fn end_turn(&mut self) -> String {
        let orders = std::mem::replace(&mut self.pending_orders, turn::TurnOrders::new());
        let events = turn::TurnProcessor::resolve(&mut self.state, &orders);
        serde_json::to_string(&events).unwrap()
    }

    /// Get all walkable hexes as JSON (for rendering).
    pub fn get_map_hexes(&self) -> String {
        let hexes: Vec<(i32, i32)> = self.state.map.all_hexes()
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&hexes).unwrap()
    }

    /// Get obstacles as JSON.
    pub fn get_obstacles(&self) -> String {
        let obstacles: Vec<(i32, i32)> = self.state.map.obstacles
            .iter()
            .map(|h| (h.q, h.r))
            .collect();
        serde_json::to_string(&obstacles).unwrap()
    }

    /// Get valid move targets for a unit (Phase 0: adjacent hexes).
    pub fn get_move_targets(&self, unit_id: unit::UnitId) -> String {
        let targets: Vec<(i32, i32)> = if let Some(unit) = self.state.get_unit(unit_id) {
            unit.pos.neighbors()
                .iter()
                .filter(|h| {
                    self.state.map.is_walkable(h) && !self.state.is_occupied(h)
                })
                .map(|h| (h.q, h.r))
                .collect()
        } else {
            vec![]
        };
        serde_json::to_string(&targets).unwrap()
    }
}
```

---

## WASM Bindings

### `crates/wasm/src/lib.rs`

```rust
use wasm_bindgen::prelude::*;
use hexabellum_core::GameEngine;

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
```

### `crates/wasm/Cargo.toml`

```toml
[package]
name = "hexabellum-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
hexabellum-core = { path = "../core" }
wasm-bindgen = "0.2"
serde_json = "1"
```

### Root `Cargo.toml`

```toml
[workspace]
members = [
    "crates/core",
    "crates/wasm",
]
resolver = "2"
```

---

## Client (TypeScript + PixiJS)

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

### `web/src/game/bridge.ts`

```typescript
import init, { WasmGame } from '../wasm/pkg/hexabellum_wasm';

let game: WasmGame | null = null;

export interface HexCoord {
  q: number;
  r: number;
}

export interface UnitData {
  id: number;
  kind: string;
  team: number;
  pos: HexCoord;
  hp: number;
  max_hp: number;
}

export interface GameState {
  round: number;
  phase: string;
  units: Record<number, UnitData>;
  winner: number | null;
}

export interface GameEvent {
  type: string;
  [key: string]: any;
}

export async function initGame(): Promise<void> {
  await init();
  game = new WasmGame();
}

export function getState(): GameState {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_state());
}

export function setMoveOrder(unitId: number, q: number, r: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_move_order(unitId, q, r);
}

export function endTurn(): GameEvent[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.end_turn());
}

export function getMapHexes(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_map_hexes()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getObstacles(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_obstacles()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getMoveTargets(unitId: number): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_move_targets(unitId)).map(([q, r]: [number, number]) => ({ q, r }));
}
```

### `web/src/game/renderer.ts`

```typescript
import * as PIXI from 'pixi.js';
import { HexCoord, UnitData, GameState } from './bridge';

// Hex geometry constants
const HEX_SIZE = 30; // radius in pixels

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer: PIXI.Container;
  private unitLayer: PIXI.Container;
  private overlayLayer: PIXI.Container;

  private hexGraphics: Map<string, PIXI.Graphics> = new Map();
  private unitSprites: Map<number, PIXI.Graphics> = new Map();

  constructor(canvas: HTMLCanvasElement) {
    this.app = new PIXI.Application({
      view: canvas,
      width: window.innerWidth,
      height: window.innerHeight,
      backgroundColor: 0x1a1a2e,
      antialias: true,
    });

    this.hexLayer = new PIXI.Container();
    this.unitLayer = new PIXI.Container();
    this.overlayLayer = new PIXI.Container();

    this.app.stage.addChild(this.hexLayer);
    this.app.stage.addChild(this.overlayLayer);
    this.app.stage.addChild(this.unitLayer);
  }

  /** Convert axial hex coords to pixel position. */
  hexToPixel(q: number, r: number): { x: number; y: number } {
    const x = HEX_SIZE * (Math.sqrt(3) * q + (Math.sqrt(3) / 2) * r);
    const y = HEX_SIZE * (3 / 2) * r;
    return {
      x: x + this.app.screen.width / 2,
      y: y + this.app.screen.height / 2,
    };
  }

  /** Draw the hex grid. */
  drawMap(hexes: HexCoord[], obstacles: HexCoord[]): void {
    this.hexLayer.removeChildren();
    this.hexGraphics.clear();

    const obstacleSet = new Set(obstacles.map(h => `${h.q},${h.r}`));

    for (const hex of hexes) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);
      const isObstacle = obstacleSet.has(`${hex.q},${hex.r}`);

      const g = new PIXI.Graphics();

      // Draw hexagon
      g.moveTo(x + HEX_SIZE, y);
      for (let i = 1; i <= 6; i++) {
        const angle = (Math.PI / 3) * i;
        g.lineTo(
          x + HEX_SIZE * Math.cos(angle),
          y + HEX_SIZE * Math.sin(angle)
        );
      }
      g.closePath();

      if (isObstacle) {
        g.fill({ color: 0x2d2d44 });
        g.stroke({ color: 0x444466, width: 1 });
      } else {
        g.fill({ color: 0x16213e });
        g.stroke({ color: 0x0f3460, width: 1 });
      }

      this.hexLayer.addChild(g);
      this.hexGraphics.set(`${hex.q},${hex.r}`, g);
    }
  }

  /** Draw units. */
  drawUnits(state: GameState): void {
    this.unitLayer.removeChildren();
    this.unitSprites.clear();

    for (const [id, unit] of Object.entries(state.units)) {
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

      const g = new PIXI.Graphics();

      // Team colors
      const color = unit.team === 0 ? 0x4fc3f7 : 0xef5350;

      // Draw unit as circle
      g.circle(x, y, HEX_SIZE * 0.5);
      g.fill({ color });
      g.stroke({ color: 0xffffff, width: 2 });

      // HP bar
      const hpWidth = HEX_SIZE * 0.8;
      const hpHeight = 4;
      const hpX = x - hpWidth / 2;
      const hpY = y - HEX_SIZE * 0.7;
      const hpRatio = unit.hp / unit.max_hp;

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x333333 });
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x4caf50 : 0xff9800 });

      this.unitLayer.addChild(g);
      this.unitSprites.set(Number(id), g);
    }
  }

  /** Highlight valid move targets. */
  drawMoveTargets(targets: HexCoord[]): void {
    this.overlayLayer.removeChildren();

    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);

      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.3);
      g.fill({ color: 0x4caf50, alpha: 0.5 });

      this.overlayLayer.addChild(g);
    }
  }

  /** Clear move highlights. */
  clearMoveTargets(): void {
    this.overlayLayer.removeChildren();
  }

  /** Highlight selected unit. */
  highlightUnit(unitId: number | null): void {
    // Reset all units
    // (In Phase 0, we just redraw)
  }

  getStage(): PIXI.Container {
    return this.app.stage;
  }

  getApp(): PIXI.Application {
    return this.app;
  }
}
```

### `web/src/game/input.ts`

```typescript
import * as PIXI from 'pixi.js';
import { HexRenderer } from './renderer';
import { getState, setMoveOrder, getMoveTargets, endTurn } from './bridge';

export class InputHandler {
  private selectedUnit: number | null = null;
  private renderer: HexRenderer;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  private setupListeners(): void {
    const app = this.renderer.getApp();
    const stage = this.renderer.getStage();

    stage.eventMode = 'static';

    stage.on('pointerdown', (event: PIXI.FederatedPointerEvent) => {
      const pos = event.global;
      this.handleClick(pos.x, pos.y);
    });
  }

  private handleClick(x: number, y: number): void {
    const state = getState();

    // Find clicked hex
    const hex = this.pixelToHex(x, y);
    if (!hex) return;

    // Check if clicked on a unit
    for (const [id, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r) {
        if (unit.team === 0) { // Player team
          this.selectedUnit = Number(id);
          const targets = getMoveTargets(Number(id));
          this.renderer.drawMoveTargets(targets);
          this.renderer.drawUnits(state);
          return;
        }
      }
    }

    // If a unit is selected, try to move
    if (this.selectedUnit !== null) {
      const success = setMoveOrder(this.selectedUnit, hex.q, hex.r);
      if (success) {
        this.renderer.clearMoveTargets();
        // Could show a "move planned" indicator here
      }
    }
  }

  /** Convert pixel position to hex coords. */
  private pixelToHex(x: number, y: number): { q: number; r: number } | null {
    const HEX_SIZE = 30;
    const app = this.renderer.getApp();

    // Offset to center
    const cx = x - app.screen.width / 2;
    const cy = y - app.screen.height / 2;

    // Pixel to fractional axial
    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / HEX_SIZE;
    const r = ((2 / 3) * cy) / HEX_SIZE;

    // Round to nearest hex
    return this.hexRound(q, r);
  }

  private hexRound(q: number, r: number): { q: number; r: number } {
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
    this.renderer.clearMoveTargets();

    // Redraw with new state
    const state = getState();
    this.renderer.drawUnits(state);

    console.log("Round events:", events);
  }

  getSelectedUnit(): number | null {
    return this.selectedUnit;
  }
}
```

### `web/src/main.ts`

```typescript
import { initGame, getState, getMapHexes, getObstacles } from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';

async function main() {
  // Initialize WASM
  await initGame();

  // Create canvas
  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;

  // Create renderer
  const renderer = new HexRenderer(canvas);

  // Draw initial state
  const hexes = getMapHexes();
  const obstacles = getObstacles();
  const state = getState();

  renderer.drawMap(hexes, obstacles);
  renderer.drawUnits(state);

  // Create input handler
  const input = new InputHandler(renderer);

  // Add "End Turn" button
  const endTurnBtn = document.getElementById('end-turn') as HTMLButtonElement;
  endTurnBtn.addEventListener('click', () => {
    input.endTurn();
  });

  // Add round counter
  const roundDisplay = document.getElementById('round') as HTMLElement;
  const updateRound = () => {
    const currentState = getState();
    roundDisplay.textContent = `Round ${currentState.round}`;
  };
  updateRound();

  // Update round display after each turn
  endTurnBtn.addEventListener('click', updateRound);

  console.log("Hexabellum Phase 0 initialized!");
}

main().catch(console.error);
```

### `web/index.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Hexabellum - Phase 0</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      background: #0a0a1a;
      color: #eee;
      font-family: 'Segoe UI', sans-serif;
      overflow: hidden;
    }
    #game-canvas {
      display: block;
      width: 100vw;
      height: 100vh;
    }
    #hud {
      position: fixed;
      top: 10px;
      left: 10px;
      z-index: 10;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }
    #round {
      font-size: 18px;
      font-weight: bold;
      color: #4fc3f7;
    }
    #end-turn {
      padding: 12px 24px;
      background: #0f3460;
      color: #4fc3f7;
      border: 2px solid #4fc3f7;
      border-radius: 8px;
      font-size: 16px;
      cursor: pointer;
      transition: all 0.2s;
    }
    #end-turn:hover {
      background: #4fc3f7;
      color: #0a0a1a;
    }
    #instructions {
      position: fixed;
      bottom: 10px;
      left: 10px;
      font-size: 14px;
      color: #888;
    }
  </style>
</head>
<body>
  <div id="hud">
    <div id="round">Round 0</div>
    <button id="end-turn">End Turn</button>
  </div>
  <canvas id="game-canvas"></canvas>
  <div id="instructions">
    Click a blue unit to select → Click a hex to plan move → End Turn to resolve
  </div>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

---

## Build Scripts

### `Makefile`

```makefile
.PHONY: build-wasm dev clean

# Build WASM package
build-wasm:
	cd crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg

# Start dev server
dev: build-wasm
	cd web && npm run dev

# Build for production
build: build-wasm
	cd web && npm run build

# Clean
clean:
	rm -rf web/src/wasm/pkg
	rm -rf web/dist
	cargo clean
```

---

## Acceptance Criteria

Phase 0 is **done** when:

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `wasm-pack build` succeeds without errors | ☐ |
| 2 | Browser loads the page and shows hex grid | ☐ |
| 3 | Two heroes visible (blue left, red right) | ☐ |
| 4 | Obstacles render in dark color | ☐ |
| 5 | Clicking blue hero highlights valid move targets | ☐ |
| 6 | Clicking a highlighted hex plans the move | ☐ |
| 7 | Clicking "End Turn" moves the hero | ☐ |
| 8 | Round counter increments | ☐ |
| 9 | Can repeat the loop indefinitely | ☐ |
| 10 | No console errors | ☐ |

---

## What is NOT in Phase 0

- ❌ Pathfinding (move is direct, no A* yet)
- ❌ AP system
- ❌ Initiative
- ❌ Combat / HP damage
- ❌ AI / bots
- ❌ Fog of war
- ❌ Multiple units per team
- ❌ Server / networking
- ❌ Timer
- ❌ Sound / animations

These come in Phase 1+.

---

## Phase 1 Preview (Next Slice)

Once Phase 0 works, Phase 1 adds:
- **Pathfinding** (A* on hex grid)
- **AP system** (movement costs AP)
- **Multiple units** (2-3 heroes per team)
- **Combat** (attack action, HP reduction, death)
- **Simple AI** (enemy heroes move toward player)
- **Turn timer** (optional, configurable)

---

## Getting Started

```bash
# 1. Install Rust + WASM toolchain
curl https://sh.rustup.rs -sSf | sh
cargo install wasm-pack

# 2. Install Node.js + npm
# (use your preferred method)

# 3. Clone / create project structure
mkdir hexabellum && cd hexabellum
# ... create files as specified above ...

# 4. Build and run
make dev
```

Open `http://localhost:5173` and you should see the hex grid with two heroes.
