# Project Hexabellum

A **turn-based MOBA** with tactical combat on a hexagonal grid. Teams submit orders simultaneously, and a deterministic engine resolves each round — blending MOBA strategy (lanes, minions, towers, objectives) with the clarity of turn-based tactics.

## Overview

Hexabellum starts as a small MVP focused on the core battle loop, built on an architecture designed to scale into:

- 5v5 synchronous multiplayer (up to 10 players)
- Gold / XP / leveling and a minimal item & shop system
- Bases, respawning, and objective-driven victory conditions
- Replays, matchmaking, and ranking in the future

### Core Gameplay Characteristics

- Team-based combat with player-controlled heroes
- Automatic minions, towers, and neutral units
- Hexagonal grid maps with obstacles and lane structure
- Simultaneous order submission, resolved deterministically by initiative
- Action Point (AP)-limited orders: move, attack, repair, abilities
- Simplified fog of war, expanding to true line-of-sight vision
- Seeded, fully deterministic simulation (same input → same output)
- Browser-based experience with Rust server authority

## Architecture

The project is divided into four layers:

| Layer | Responsibility |
|-------|----------------|
| **Game** | Data-driven content: heroes, arenas, units, minions, neutrals, game modes |
| **Engine** | Deterministic simulation: state, turns, pathfinding, combat, effects, AI, vision |
| **Client** | Presentation & input only — never owns game truth |
| **Services** | Server-side: match rooms, WebSockets, persistence, replays (post-MVP focus) |

Key design principles:

1. **Deterministic simulation** — seeded RNG (`rand_chacha`), stable sorting, no system time or hashmap ordering inside the simulation
2. **Server authority** — clients submit intentions; the engine resolves
3. **Data-driven content** — new heroes/maps/abilities without touching engine code
4. **Event-driven battle history** — every meaningful change emits an event (drives UI, battle log, replays)
5. **Simulation/presentation separation** — the engine has no dependency on graphics, DOM, or network

State synchronization uses a **snapshot + events** model: full state at planning start, orders submitted by clients, events + updated state after resolution.

## Tech Stack

- **Shared core:** `hexabellum-core` — pure Rust simulation crate, compiles to native (server/tests) and WASM (browser)
- **WASM bridge:** `hexabellum-wasm` — `wasm-bindgen`, `serde-wasm-bindgen`/`tsify`, `web-sys`
- **Protocol:** `hexabellum-protocol` — shared client/server message DTOs
- **Server:** Rust — `tokio`, `axum`, `serde`, WebSockets, `tracing`, `thiserror`
- **Client:** TypeScript + Vite + PixiJS (2D WebGL hex board) + DOM HUD

### Repository Layout

```text
hexabellum/
  crates/
    core/             # hex, state, rules, turn, resolution, effects, ai, vision, replay
    protocol/         # message types & serialization (hexabellum-protocol)
    server/           # match rooms, websockets, authoritative execution (hexabellum-server)
    wasm/             # JS/TS interop, local battle runner (hexabellum-wasm)
  web/                # Vite + TypeScript + PixiJS client
    src/
      ui/ renderer/ net/ wasm/
  docs/               # architecture & phase specifications
```

## Development Roadmap

The project is built as a series of **vertical slices** — each phase proves the full pipeline end-to-end rather than building isolated layers. See the specs in [`docs/`](docs/).

| Phase | Slice | Deliverable |
|-------|-------|-------------|
| [0](docs/phase-0.md) | Foundation / first playable slice | Hex grid renders in browser via WASM; select hero, move, end turn |
| [1](docs/phase-1.md) | Combat & AI | 3v3 battle: A* pathfinding, AP, attacks, initiative, AI opponent, win/loss |
| [2](docs/phase-2.md) | MOBA elements | Minion spawning & pushing, towers with auto-attack, fog of war, turn timer |
| [3](docs/phase-3.md) | Authoritative server | Rust server, WebSocket protocol, match rooms, server-run rounds |
| [4](docs/phase-4.md) | Abilities & depth | Hero abilities, repair, neutral camps, lane-aware minions, line-of-sight vision |
| [5](docs/phase-5.md) | 5v5 readiness | One hero per player, shared team vision, AI backfill for empty slots |
| [6](docs/phase-6.md) | Economy & progression | Gold, XP, hero levels, minimal passive items, validated server-side |
| [7](docs/phase-7.md) | MOBA macro loop | Base, respawn, position-based shopping, objectives, destroy-the-Core victory |

Full architecture details: [`docs/overview.md`](docs/overview.md).

## Getting Started

```bash
# Build the WASM package
make build-wasm

# Run the web client
make dev

# Run the server (Phase 3+)
cargo run -p hexabellum-server

# Headless tests / AI-vs-AI simulation
cargo test -p hexabellum-core
```

## Testing Strategy

- **Unit tests:** hex distance/neighbors, pathfinding, AP validation, initiative sorting, damage, repair, fog radius
- **Simulation tests:** AI vs AI battles, fixed-seed runs, golden-master replays, edge cases (contested hexes, death-before-acting, invalid orders, timeout fallback)
- **Determinism tests:** identical seed + orders must always produce identical final state hash, event sequence, positions, and winner
