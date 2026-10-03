import init, { WasmGame, WasmTutorialSession } from '../wasm/pkg/hexabellum_wasm';

let game: WasmGame | null = null;

export function createTutorialSession(scenarioIdOrJson: string): WasmTutorialSession {
  return new WasmTutorialSession(scenarioIdOrJson);
}

export interface HexCoord {
  q: number;
  r: number;
}

export type UnitKind = 'Hero' | 'Minion' | 'Tower' | 'Spawner' | 'SpawnerTower' | 'Neutral' | 'NeutralGuardian';

export interface UnitData {
  id: number;
  kind: UnitKind;
  team: number;
  pos: HexCoord;
  hp: number;
  max_hp: number;
  ap: number;
  max_ap: number;
  energy?: number;
  max_energy?: number;
  initiative: number;
  attack_damage: number;
  attack_range: number;
  vision_range: number;
  cooldowns?: Record<string, number>;
  statuses?: { id: string; remaining_rounds: number; attack_damage_mod: number }[];
  lane_id?: string | null;
  spawn_interval?: number;
  spawn_counter: number;
}

export interface GameState {
  round: number;
  phase: 'Planning' | 'Resolution' | 'MatchEnd';
  units: Record<number, UnitData>;
  winner: number | null;
}

export type GameEvent =
  | { type: 'RoundStarted'; round: number }
  | { type: 'UnitSpawned'; unit_id: number; unit_kind: UnitKind; team: number; pos: HexCoord; spawner_id: number }
  | { type: 'UnitMoved'; unit_id: number; from: HexCoord; to: HexCoord; path: HexCoord[]; ap_spent: number }
  | { type: 'UnitAttacked'; attacker_id: number; target_id: number; damage: number; target_hp_remaining: number }
  | { type: 'TowerAttacked'; tower_id: number; target_id: number; damage: number; target_hp_remaining: number }
  | { type: 'SpellCast'; caster_id: number; spell_id: string; target: any }
  | { type: 'HealApplied'; caster_id: number; target_id: number; amount: number; target_hp_remaining: number }
  | { type: 'StructureRepaired'; repairer_id: number; target_id: number; amount: number; target_hp_remaining: number }
  | { type: 'StatusApplied'; unit_id: number; status_id: string; duration_rounds: number }
  | { type: 'StatusExpired'; unit_id: number; status_id: string }
  | { type: 'NeutralCampCleared'; camp_id: string; killer_team: number }
  | { type: 'TeamBuffApplied'; team: number; buff_id: string; duration_rounds: number }
  | { type: 'UnitDied'; unit_id: number; unit_kind: UnitKind; killed_by: number }
  | { type: 'UnitWaited'; unit_id: number }
  | { type: 'FogUpdated'; team: number; visible_hexes: HexCoord[] }
  | { type: 'RoundEnded'; round: number }
  | { type: 'MatchEnded'; winner: number | null };

export interface MoveTarget {
  q: number;
  r: number;
  cost: number;
}

export async function initGame(): Promise<void> {
  await init();
  game = new WasmGame();
}

export function getState(): GameState {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_state());
}

export function getPlayerState(team: number = 0): GameState {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_player_state(team));
}

export function getPlayerFog(): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.get_player_fog()).map(([q, r]: [number, number]) => ({ q, r }));
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

export function findPath(fromQ: number, fromR: number, toQ: number, toR: number): HexCoord[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.find_path(fromQ, fromR, toQ, toR)).map(
    ([q, r]: [number, number]) => ({ q, r })
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

export function getPendingOrders(): string {
  if (!game) throw new Error("Game not initialized");
  return game.get_pending_orders();
}

export function allUnitsOrdered(): boolean {
  if (!game) throw new Error("Game not initialized");
  return game.all_units_ordered();
}

export function endTurn(): GameEvent[] {
  if (!game) throw new Error("Game not initialized");
  return JSON.parse(game.end_turn());
}

export function clearOrders(unitId: number): void {
  if (!game) throw new Error("Game not initialized");
  game.clear_orders(BigInt(unitId));
}

export function restart(): void {
  if (!game) throw new Error("Game not initialized");
  game.restart();
}
