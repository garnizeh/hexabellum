import { NetworkBridge, ConnectionState } from './net';
import {
  SnapshotDto,
  HexDto,
  OrderDto,
  SanitizedGameEvent,
  ProtocolErrorCode,
  SpellTargetDto,
  MatchPhaseDto,
  PlayerLobbyDto,
  HeroDto,
  RosterEntryDto,
  HeroDefId,
} from './types';
import { GameState, UnitData, HexCoord, UnitKind } from './bridge';
import {
  findPath as hexFindPath,
  getMoveTargets as hexGetMoveTargets,
  getAttackTargets as hexGetAttackTargets,
  getRepairTargets as hexGetRepairTargets,
  getSpellTargets as hexGetSpellTargets,
  SpellTargetingResult,
} from './hex_math';

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
      attack_damage: u.attack_damage ?? (u.kind === 'Tower' ? 30 : u.kind === 'Hero' ? 20 : 8),
      attack_range: u.attack_range,
      vision_range: u.vision_range,
      spawn_counter: 0,
      energy: u.energy ?? 0,
      max_energy: u.max_energy ?? 0,
      cooldowns: u.cooldowns ?? {},
      statuses: u.statuses ?? [],
      lane_id: u.lane_id,
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

export interface LobbyState {
  matchId: string;
  phase: MatchPhaseDto;
  players: PlayerLobbyDto[];
  heroPools: Record<number, HeroDto[]>;
  countdownMs?: number | null;
}

export interface ClientSessionEvents {
  onConnectionChange: (state: ConnectionState) => void;
  onMatchJoined: (matchId: string, team: number, snapshot: SnapshotDto) => void;
  onLobbyUpdated: (lobby: LobbyState) => void;
  onHeroSelected: (playerId: string, team: number, heroDefId: HeroDefId) => void;
  onMatchStarting: (round: number, snapshot: SnapshotDto) => void;
  onRoundStarted: (round: number, deadlineUnixMs: number, snapshot: SnapshotDto) => void;
  onOrdersAccepted: (round: number) => void;
  onOrderRejected: (round: number, code: ProtocolErrorCode, reason: string) => void;
  onEarlyResolutionTriggered: (round: number, resolutionUnixMs: number) => void;
  onRoundResolved: (
    round: number,
    events: SanitizedGameEvent[],
    snapshot: SnapshotDto,
    stateHash?: string | null
  ) => void;
  onPlayerConnectionUpdated: (playerId: string, connected: boolean, isAiControlled: boolean) => void;
  onMatchEnded: (winner: number | null, snapshot: SnapshotDto, stateHash?: string | null) => void;
  onOpponentStatus?: (online: boolean) => void;
  onError: (msg: string) => void;
}

export class ClientSession {
  private net: NetworkBridge;
  private currentSnapshot: SnapshotDto | null = null;
  private lobbyState: LobbyState | null = null;
  private currentTeam: number = 0;
  private stagedOrders = new Map<number, OrderDto>();
  private events: Partial<ClientSessionEvents> = {};
  private isPvAi: boolean = false;

  constructor() {
    this.net = new NetworkBridge();
    this.setupNetworkCallbacks();
  }

  getIsPvAI(): boolean {
    return this.isPvAi;
  }

  setIsPvAI(val: boolean): void {
    this.isPvAi = val;
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

  getLobbyState(): LobbyState | null {
    return this.lobbyState;
  }

  getGameState(): GameState | null {
    return this.currentSnapshot ? snapshotToGameState(this.currentSnapshot) : null;
  }

  getControlledUnitIds(): number[] {
    return this.currentSnapshot?.controlled_units ?? [];
  }

  getPrimaryControlledUnitId(): number | null {
    const list = this.currentSnapshot?.controlled_units;
    return list && list.length > 0 ? list[0] : null;
  }

  isMyControlledUnit(unitId: number): boolean {
    if (!this.currentSnapshot) return true;
    if (this.currentSnapshot.controlled_units.length === 0) return true;
    return this.currentSnapshot.controlled_units.includes(unitId);
  }

  getMyHero(): UnitData | null {
    const primaryId = this.getPrimaryControlledUnitId();
    if (primaryId === null) return null;
    const state = this.getGameState();
    return state?.units[primaryId] ?? null;
  }

  getRoster(): RosterEntryDto[] {
    return this.currentSnapshot?.roster ?? [];
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
    this.isPvAi = enableAiTeam1;
    const res = await fetch('/api/matches', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        enable_ai_team_1: enableAiTeam1,
        turn_duration_secs: turnDurationSecs,
      }),
    });
    if (!res.ok) {
      let detail = res.statusText;
      try {
        const body = await res.json();
        if (body?.message) detail = body.message;
      } catch {}
      throw new Error(`Failed to create match (${res.status}): ${detail}`);
    }
    const data = await res.json();
    return data.match_id;
  }

  async create5v5Match(enableAiTeam1: boolean = true, turnDurationSecs: number = 30): Promise<string> {
    this.isPvAi = enableAiTeam1;
    const res = await fetch('/api/matches', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        enable_ai_team_1: enableAiTeam1,
        turn_duration_secs: turnDurationSecs,
        players_per_team: 5,
        heroes_per_team: 5,
        map_radius: 8,
        skip_draft: false,
      }),
    });
    if (!res.ok) {
      let detail = res.statusText;
      try {
        const body = await res.json();
        if (body?.message) detail = body.message;
      } catch {}
      throw new Error(`Failed to create 5v5 match (${res.status}): ${detail}`);
    }
    const data = await res.json();
    return data.match_id;
  }

  joinMatch(matchId: string): void {
    this.stagedOrders.clear();
    this.net.connect(matchId);
  }

  selectHero(heroDefId: HeroDefId): void {
    this.net.selectHero(heroDefId);
  }

  setReady(ready: boolean): void {
    this.net.setReady(ready);
  }

  cancelOrders(): void {
    if (this.currentSnapshot) {
      this.net.cancelOrders(this.currentSnapshot.round);
    }
  }

  getStagedCount(): number {
    return this.stagedOrders.size;
  }

  stageWaitOrdersForControlled(): void {
    if (!this.currentSnapshot) return;
    for (const unitId of this.currentSnapshot.controlled_units) {
      if (!this.stagedOrders.has(unitId)) {
        this.stageWaitOrder(unitId);
      }
    }
  }

  stageMoveOrder(unitId: number, q: number, r: number): boolean {
    if (!this.isMyControlledUnit(unitId)) return false;
    const existing = this.stagedOrders.get(unitId);
    this.stagedOrders.set(unitId, {
      unit_id: unitId,
      move_target: { q, r },
      action: existing?.action ?? { type: 'Wait' },
    });
    return true;
  }

  stageAttackOrder(unitId: number, targetId: number): boolean {
    if (!this.isMyControlledUnit(unitId)) return false;
    if (!this.currentSnapshot) return false;
    const unit = this.currentSnapshot.units.find(u => u.id === unitId);
    if (!unit || unit.hp === 0) return false;

    const existing = this.stagedOrders.get(unitId);
    let moveCost = 0;
    if (existing?.move_target) {
      const path = this.findPath(unit.pos.q, unit.pos.r, existing.move_target.q, existing.move_target.r);
      moveCost = path.length > 1 ? path.length - 1 : 0;
    }
    // Attack costs 1 AP
    if (moveCost + 1 > unit.ap) {
      return false;
    }

    this.stagedOrders.set(unitId, {
      unit_id: unitId,
      move_target: existing?.move_target ?? null,
      action: { type: 'Attack', target_id: targetId },
    });
    return true;
  }

  stageCastOrder(unitId: number, spellId: string, target: SpellTargetDto): boolean {
    if (!this.isMyControlledUnit(unitId)) return false;
    if (!this.currentSnapshot) return false;
    const unit = this.currentSnapshot.units.find(u => u.id === unitId);
    if (!unit || unit.hp === 0) return false;

    const existing = this.stagedOrders.get(unitId);
    let moveCost = 0;
    if (existing?.move_target) {
      const path = this.findPath(unit.pos.q, unit.pos.r, existing.move_target.q, existing.move_target.r);
      moveCost = path.length > 1 ? path.length - 1 : 0;
    }
    if (moveCost + 1 > unit.ap) {
      return false;
    }

    this.stagedOrders.set(unitId, {
      unit_id: unitId,
      move_target: existing?.move_target ?? null,
      action: { type: 'Cast', spell_id: spellId, target },
    });
    return true;
  }

  stageRepairOrder(unitId: number, targetId: number): boolean {
    if (!this.isMyControlledUnit(unitId)) return false;
    if (!this.currentSnapshot) return false;
    const unit = this.currentSnapshot.units.find(u => u.id === unitId);
    if (!unit || unit.hp === 0) return false;

    const existing = this.stagedOrders.get(unitId);
    let moveCost = 0;
    if (existing?.move_target) {
      const path = this.findPath(unit.pos.q, unit.pos.r, existing.move_target.q, existing.move_target.r);
      moveCost = path.length > 1 ? path.length - 1 : 0;
    }
    if (moveCost + 1 > unit.ap) {
      return false;
    }

    this.stagedOrders.set(unitId, {
      unit_id: unitId,
      move_target: existing?.move_target ?? null,
      action: { type: 'Repair', target_id: targetId },
    });
    return true;
  }

  stageWaitOrder(unitId: number): boolean {
    if (!this.isMyControlledUnit(unitId)) return false;
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

  getVisionBlockers(): Set<string> {
    if (this.currentSnapshot?.map?.obstacles?.length) {
      return new Set(this.currentSnapshot.map.obstacles.map(h => `${h.q},${h.r}`));
    }
    // Radius 8 arena features: 6 tactical mid-lane vision blockers
    return new Set(['0,-2', '0,2', '-2,-3', '2,-3', '-3,2', '3,-2']);
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

    let moveCost = 0;
    if (unit.pos.q !== fromQ || unit.pos.r !== fromR) {
      const path = this.findPath(unit.pos.q, unit.pos.r, fromQ, fromR);
      moveCost = path.length > 1 ? path.length - 1 : 0;
    }
    if (moveCost + 1 > unit.ap) {
      return [];
    }

    const state = snapshotToGameState(this.currentSnapshot);
    const blockers = this.getVisionBlockers();
    const visibleHexes = this.currentSnapshot.visible_hexes
      ? new Set(this.currentSnapshot.visible_hexes.map(h => `${h.q},${h.r}`))
      : undefined;
    return hexGetAttackTargets(
      { q: fromQ, r: fromR },
      unit.attack_range,
      state.units,
      unit.team,
      visibleHexes,
      blockers
    );
  }

  getRepairTargets(unitId: number, fromQ: number, fromR: number): number[] {
    if (!this.currentSnapshot) return [];
    const unit = this.currentSnapshot.units.find(u => u.id === unitId);
    if (!unit || unit.hp === 0) return [];

    let moveCost = 0;
    if (unit.pos.q !== fromQ || unit.pos.r !== fromR) {
      const path = this.findPath(unit.pos.q, unit.pos.r, fromQ, fromR);
      moveCost = path.length > 1 ? path.length - 1 : 0;
    }
    if (moveCost + 1 > unit.ap) {
      return [];
    }

    const state = snapshotToGameState(this.currentSnapshot);
    return hexGetRepairTargets(
      { q: fromQ, r: fromR },
      state.units,
      unit.team
    );
  }

  getSpellTargets(unitId: number, spellId: string, fromQ: number, fromR: number): SpellTargetingResult {
    if (!this.currentSnapshot) {
      return { validUnitIds: [], obstructedUnitIds: [], isSelfOnly: false };
    }
    const unit = this.currentSnapshot.units.find(u => u.id === unitId);
    if (!unit || unit.hp === 0) {
      return { validUnitIds: [], obstructedUnitIds: [], isSelfOnly: false };
    }

    let moveCost = 0;
    if (unit.pos.q !== fromQ || unit.pos.r !== fromR) {
      const path = this.findPath(unit.pos.q, unit.pos.r, fromQ, fromR);
      moveCost = path.length > 1 ? path.length - 1 : 0;
    }
    if (moveCost + 1 > unit.ap) {
      return { validUnitIds: [], obstructedUnitIds: [], isSelfOnly: false };
    }

    const state = snapshotToGameState(this.currentSnapshot);
    const blockers = this.getVisionBlockers();
    const virtualCaster: UnitData = {
      ...state.units[unitId],
      pos: { q: fromQ, r: fromR },
    };
    const visibleHexes = this.currentSnapshot?.visible_hexes
      ? new Set<string>(this.currentSnapshot.visible_hexes.map((h: HexDto) => `${h.q},${h.r}`))
      : undefined;
    return hexGetSpellTargets(virtualCaster, spellId, state.units, blockers, visibleHexes);
  }

  private setupNetworkCallbacks(): void {
    this.net.setCallbacks({
      onConnectionChange: (state) => {
        this.events.onConnectionChange?.(state);
      },
      onMatchJoined: (matchId, team, snapshot) => {
        this.currentSnapshot = snapshot;
        this.currentTeam = team;
        this.events.onMatchJoined?.(matchId, team, snapshot);
      },
      onLobbyUpdated: (matchId, phase, players, heroPools, countdownMs) => {
        this.lobbyState = {
          matchId,
          phase,
          players,
          heroPools,
          countdownMs,
        };
        this.events.onLobbyUpdated?.(this.lobbyState);
      },
      onHeroSelected: (playerId, team, heroDefId) => {
        this.events.onHeroSelected?.(playerId, team, heroDefId);
      },
      onMatchStarting: (round, snapshot) => {
        this.currentSnapshot = snapshot;
        this.events.onMatchStarting?.(round, snapshot);
      },
      onRoundStarted: (round, deadlineUnixMs, snapshot) => {
        this.currentSnapshot = snapshot;
        this.stagedOrders.clear();
        this.events.onRoundStarted?.(round, deadlineUnixMs, snapshot);
      },
      onOrdersAccepted: (round) => {
        this.events.onOrdersAccepted?.(round);
      },
      onOrderRejected: (round, code, reason) => {
        this.events.onOrderRejected?.(round, code, reason);
      },
      onEarlyResolutionTriggered: (round, resolutionUnixMs) => {
        this.events.onEarlyResolutionTriggered?.(round, resolutionUnixMs);
      },
      onRoundResolved: (round, events, snapshot, stateHash) => {
        this.currentSnapshot = snapshot;
        this.stagedOrders.clear();
        this.events.onRoundResolved?.(round, events, snapshot, stateHash);
      },
      onPlayerConnectionUpdated: (playerId, connected, isAiControlled) => {
        this.events.onPlayerConnectionUpdated?.(playerId, connected, isAiControlled);
      },
      onMatchEnded: (winner, snapshot, stateHash, _totalRounds) => {
        this.currentSnapshot = snapshot;
        this.events.onMatchEnded?.(winner, snapshot, stateHash);
      },
      onOpponentStatus: (online) => {
        this.events.onOpponentStatus?.(online);
      },
      onError: (msg) => {
        this.events.onError?.(msg);
      },
    });
  }
}
