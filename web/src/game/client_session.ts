import { NetworkBridge, ConnectionState } from './net';
import {
  SnapshotDto,
  OrderDto,
  SanitizedGameEvent,
  ProtocolErrorCode,
} from './types';
import { GameState, UnitData, HexCoord, UnitKind } from './bridge';
import { findPath as hexFindPath, getMoveTargets as hexGetMoveTargets, getAttackTargets as hexGetAttackTargets } from './hex_math';

export function snapshotToGameState(snapshot: SnapshotDto): GameState {
  const units: Record<number, UnitData> = {};
  for (const u of snapshot.units) {
    units[u.id] = {
      id: u.id,
      kind: u.kind as UnitKind,
      team: u.team,
      pos: { q: u.pos.q, r: u.pos.r },
      hp: u.hp,
      max_hp: u.max_hp,
      ap: u.ap,
      max_ap: u.max_ap,
      initiative: u.initiative,
      attack_damage: u.kind === 'Tower' ? 30 : u.kind === 'Hero' ? 20 : 8,
      attack_range: u.attack_range,
      vision_range: u.vision_range,
      spawn_counter: 0,
    };
  }

  let phase: 'Planning' | 'Resolution' | 'MatchEnd' = 'Planning';
  if (snapshot.phase.toLowerCase().includes('res')) {
    phase = 'Resolution';
  } else if (snapshot.phase.toLowerCase().includes('end') || snapshot.winner !== null) {
    phase = 'MatchEnd';
  }

  return {
    round: snapshot.round,
    phase,
    units,
    winner: snapshot.winner,
  };
}

export interface ClientSessionEvents {
  onConnectionChange: (state: ConnectionState) => void;
  onMatchJoined: (matchId: string, team: number, snapshot: SnapshotDto) => void;
  onRoundStarted: (round: number, deadlineUnixMs: number, snapshot: SnapshotDto) => void;
  onOrdersAccepted: (round: number) => void;
  onOrderRejected: (round: number, code: ProtocolErrorCode, reason: string) => void;
  onRoundResolved: (round: number, events: SanitizedGameEvent[], snapshot: SnapshotDto) => void;
  onMatchEnded: (winner: number | null, snapshot: SnapshotDto) => void;
  onError: (msg: string) => void;
}

export class ClientSession {
  private net: NetworkBridge;
  private currentSnapshot: SnapshotDto | null = null;
  private currentTeam: number = 0;
  private stagedOrders = new Map<number, OrderDto>();
  private events: Partial<ClientSessionEvents> = {};

  constructor() {
    this.net = new NetworkBridge();
    this.setupNetworkCallbacks();
  }

  setEvents(events: Partial<ClientSessionEvents>): void {
    this.events = events;
  }

  getNet(): NetworkBridge {
    return this.net;
  }

  getCurrentTeam(): number {
    return this.currentTeam;
  }

  getSnapshot(): SnapshotDto | null {
    return this.currentSnapshot;
  }

  getGameState(): GameState | null {
    return this.currentSnapshot ? snapshotToGameState(this.currentSnapshot) : null;
  }

  getMapHexes(): HexCoord[] {
    if (!this.currentSnapshot) return [];
    return this.currentSnapshot.map.walkable;
  }

  getObstacles(): HexCoord[] {
    if (!this.currentSnapshot) return [];
    return this.currentSnapshot.map.obstacles;
  }

  getFog(): HexCoord[] {
    if (!this.currentSnapshot) return [];
    return this.currentSnapshot.visible_hexes;
  }

  async createMatch(enableAiTeam1: boolean = true, turnDurationSecs: number = 30): Promise<string> {
    const res = await fetch('/api/matches', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        enable_ai_team_1: enableAiTeam1,
        turn_duration_secs: turnDurationSecs,
      }),
    });
    if (!res.ok) throw new Error(`Failed to create match: ${res.statusText}`);
    const data = await res.json();
    return data.match_id;
  }

  joinMatch(matchId: string): void {
    this.stagedOrders.clear();
    this.net.connect(matchId);
  }

  stageMoveOrder(unitId: number, q: number, r: number): boolean {
    const existing = this.stagedOrders.get(unitId);
    this.stagedOrders.set(unitId, {
      unit_id: unitId,
      move_target: { q, r },
      action: existing?.action ?? { type: 'Wait' },
    });
    return true;
  }

  stageAttackOrder(unitId: number, targetId: number): boolean {
    const existing = this.stagedOrders.get(unitId);
    this.stagedOrders.set(unitId, {
      unit_id: unitId,
      move_target: existing?.move_target ?? null,
      action: { type: 'Attack', target_id: targetId },
    });
    return true;
  }

  stageWaitOrder(unitId: number): boolean {
    const existing = this.stagedOrders.get(unitId);
    this.stagedOrders.set(unitId, {
      unit_id: unitId,
      move_target: existing?.move_target ?? null,
      action: { type: 'Wait' },
    });
    return true;
  }

  clearOrder(unitId: number): void {
    this.stagedOrders.delete(unitId);
  }

  submitOrders(): void {
    if (!this.currentSnapshot) return;
    const orders: OrderDto[] = Array.from(this.stagedOrders.values());
    this.net.submitOrders(this.currentSnapshot.round, orders);
  }

  getMoveTargets(unitId: number): { q: number; r: number; cost: number }[] {
    if (!this.currentSnapshot) return [];
    const unit = this.currentSnapshot.units.find(u => u.id === unitId);
    if (!unit || unit.is_stationary || unit.hp === 0) return [];

    const obstacles = new Set(this.currentSnapshot.map.obstacles.map(h => `${h.q},${h.r}`));
    const walkable = new Set(this.currentSnapshot.map.walkable.map(h => `${h.q},${h.r}`));
    const occupied = new Set(this.currentSnapshot.units.filter(u => u.hp > 0).map(u => `${u.pos.q},${u.pos.r}`));

    return hexGetMoveTargets(unit.pos, unit.ap, obstacles, walkable, occupied);
  }

  findPath(fromQ: number, fromR: number, toQ: number, toR: number): HexCoord[] {
    if (!this.currentSnapshot) return [];
    const obstacles = new Set(this.currentSnapshot.map.obstacles.map(h => `${h.q},${h.r}`));
    const walkable = new Set(this.currentSnapshot.map.walkable.map(h => `${h.q},${h.r}`));
    return hexFindPath({ q: fromQ, r: fromR }, { q: toQ, r: toR }, obstacles, walkable) ?? [];
  }

  getAttackTargets(unitId: number, fromQ: number, fromR: number): number[] {
    if (!this.currentSnapshot) return [];
    const unit = this.currentSnapshot.units.find(u => u.id === unitId);
    if (!unit || unit.hp === 0) return [];

    const state = snapshotToGameState(this.currentSnapshot);
    const visibleHexes = new Set(this.currentSnapshot.visible_hexes.map(h => `${h.q},${h.r}`));
    return hexGetAttackTargets({ q: fromQ, r: fromR }, unit.attack_range, state.units, this.currentTeam, visibleHexes);
  }

  private setupNetworkCallbacks(): void {
    this.net.setCallbacks({
      onConnectionChange: (state) => this.events.onConnectionChange?.(state),
      onMatchJoined: (team, snapshot) => {
        this.currentTeam = team;
        this.currentSnapshot = snapshot;
        this.stagedOrders.clear();
        this.events.onMatchJoined?.(snapshot.match_id, team, snapshot);
      },
      onRoundStarted: (round, deadline, snapshot) => {
        this.currentSnapshot = snapshot;
        this.stagedOrders.clear();
        this.events.onRoundStarted?.(round, deadline, snapshot);
      },
      onOrdersAccepted: (round) => this.events.onOrdersAccepted?.(round),
      onOrderRejected: (round, code, reason) => this.events.onOrderRejected?.(round, code, reason),
      onRoundResolved: (round, events, snapshot) => {
        this.currentSnapshot = snapshot;
        this.events.onRoundResolved?.(round, events, snapshot);
      },
      onMatchEnded: (winner, snapshot) => {
        this.currentSnapshot = snapshot;
        this.events.onMatchEnded?.(winner, snapshot);
      },
      onError: (msg) => this.events.onError?.(msg),
    });
  }
}
