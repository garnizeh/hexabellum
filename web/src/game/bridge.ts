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
  ap: number;
  max_ap: number;
  initiative: number;
  attack_damage: number;
  attack_range: number;
}

export interface GameState {
  round: number;
  phase: 'Planning' | 'Resolution' | 'MatchEnd';
  units: Record<number, UnitData>;
  winner: number | null;
  next_unit_id: number;
}

export interface MoveTarget {
  q: number;
  r: number;
  cost: number;
}

export interface PendingOrder {
  unit_id: number;
  move_target: HexCoord | null;
  action: 'Wait' | { Attack: { target_id: number } };
}

export interface TurnOrders {
  orders: PendingOrder[];
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

export function getMapHexes(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_map_hexes()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getObstacles(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_obstacles()).map(([q, r]: [number, number]) => ({ q, r }));
}

export function getMoveTargets(unitId: number): MoveTarget[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_move_targets(BigInt(unitId))).map(
    ([q, r, cost]: [number, number, number]) => ({ q, r, cost })
  );
}

export function getAttackTargets(unitId: number, fromQ: number, fromR: number): number[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_attack_targets(BigInt(unitId), fromQ, fromR));
}

export function setMoveOrder(unitId: number, q: number, r: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_move_order(BigInt(unitId), q, r);
}

export function setAttackOrder(unitId: number, targetId: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_attack_order(BigInt(unitId), BigInt(targetId));
}

export function setWaitOrder(unitId: number): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.set_wait_order(BigInt(unitId));
}

export function getPendingOrders(): TurnOrders {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_pending_orders());
}

export function allUnitsOrdered(): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.all_units_ordered();
}

export function endTurn(): GameEvent[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.end_turn());
}

export function restart(): void {
  if (!game) throw new Error("Game not initialized");
  game.restart();
}

export function clearOrders(unitId: number): void {
  if (!game) throw new Error("Game not initialized");
  game.clear_orders(BigInt(unitId));
}
