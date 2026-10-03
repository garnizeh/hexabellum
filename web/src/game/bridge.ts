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
  [key: string]: unknown;
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
  // u64 crosses the boundary as BigInt
  return game.set_move_order(BigInt(unitId), q, r);
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
  return JSON.parse(game.get_move_targets(BigInt(unitId))).map(([q, r]: [number, number]) => ({ q, r }));
}
