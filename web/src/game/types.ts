export interface HexDto {
  q: number;
  r: number;
}

export type HexCoord = HexDto;

export interface StatusDto {
  id: string;
  remaining_rounds: number;
  attack_damage_mod: number;
}

export interface NeutralCampDto {
  id: string;
  pos: HexDto;
  is_alive: boolean;
  guardian_unit_id?: number | null;
}

export interface UnitDto {
  id: number;
  kind: string; // "Hero" | "Minion" | "Tower" | "Spawner" | "NeutralGuardian"
  team: number;
  pos: HexDto;
  hp: number;
  max_hp: number;
  ap: number;
  max_ap: number;
  energy: number;
  max_energy: number;
  initiative: number;
  attack_damage: number;
  attack_range: number;
  vision_range: number;
  is_stationary: boolean;
  cooldowns: Record<string, number>;
  statuses: StatusDto[];
  lane_id?: string | null;
  hero_id?: string | null;
}

export interface MapDto {
  radius: number;
  walkable: HexDto[];
  obstacles: HexDto[];
}

export type HeroDefId = string;

export type MatchPhaseDto =
  | 'Lobby'
  | 'HeroSelect'
  | 'Planning'
  | 'Resolution'
  | 'MatchEnd';

export interface PlayerLobbyDto {
  player_id: string;
  display_name: string;
  team: number;
  connected: boolean;
  ready: boolean;
  hero_def_id: HeroDefId | null;
  is_ai: boolean;
  ping_ms?: number | null;
}

export interface HeroDto {
  id: HeroDefId;
  name: string;
  role: string;
  max_hp: number;
  attack_damage: number;
  attack_range: number;
  vision_range: number;
  max_energy: number;
  spell_id: string;
  spell_name: string;
  spell_desc: string;
}

export interface RosterEntryDto {
  player_id: string | null;
  display_name: string;
  hero_def_id: HeroDefId;
  unit_id: number;
  team: number;
  connected: boolean;
  is_ai: boolean;
  orders_submitted: boolean;
  alive: boolean;
  hp: number | null;
  max_hp: number;
}

export interface SnapshotDto {
  match_id: string;
  round: number;
  phase: string;
  winner: number | null;
  map: MapDto;
  units: UnitDto[];
  visible_hexes: HexDto[];
  controlled_units: number[];
  deadline_unix_ms?: number | null;
  state_hash: string;
  neutral_camps: NeutralCampDto[];
  player_team: number;
  roster: RosterEntryDto[];
  match_phase?: MatchPhaseDto | null;
}

export type SpellTargetDto =
  | { type: 'None' }
  | { type: 'Unit'; payload: { unit_id: number } }
  | { type: 'Hex'; payload: { hex: HexDto } };

export type ActionDto =
  | { type: 'Wait' }
  | { type: 'Attack'; target_id: number }
  | { type: 'Cast'; spell_id: string; target: SpellTargetDto }
  | { type: 'Repair'; target_id: number };

export interface OrderDto {
  unit_id: number;
  move_target?: HexDto | null;
  action: ActionDto;
}

export type SanitizedGameEvent =
  | { type: 'RoundStarted'; round: number }
  | { type: 'UnitSpawned'; unit_id: number; unit_kind: string; team: number; pos: HexDto; spawner_id: number }
  | { type: 'UnitMoved'; unit_id: number; from: HexDto; to: HexDto; path: HexDto[]; ap_spent: number }
  | { type: 'UnitAttacked'; attacker_id: number; target_id: number; damage: number; target_hp_remaining: number }
  | { type: 'TowerAttacked'; tower_id: number; target_id: number; damage: number; target_hp_remaining: number }
  | { type: 'SpellCast'; caster_id: number; spell_id: string; target: SpellTargetDto }
  | { type: 'HealApplied'; caster_id: number; target_id: number; amount: number; target_hp_remaining: number }
  | { type: 'StructureRepaired'; repairer_id: number; target_id: number; amount: number; target_hp_remaining: number }
  | { type: 'StatusApplied'; unit_id: number; status_id: string; duration_rounds: number }
  | { type: 'StatusExpired'; unit_id: number; status_id: string }
  | { type: 'NeutralCampCleared'; camp_id: string; killer_team: number }
  | { type: 'TeamBuffApplied'; team: number; buff_id: string; duration_rounds: number }
  | { type: 'UnitDied'; unit_id: number; unit_kind: string; killed_by: number }
  | { type: 'UnitWaited'; unit_id: number }
  | { type: 'RoundEnded'; round: number }
  | { type: 'MatchEnded'; winner: number | null };

export type ClientMessage =
  | { type: 'Hello'; player_id: string; reconnect_token?: string | null }
  | {
      type: 'JoinMatch';
      match_id?: string;
      player_id?: string | null;
      display_name?: string | null;
      reconnect_token?: string | null;
      preferred_team?: number | null;
    }
  | { type: 'SelectHero'; hero_def_id: HeroDefId }
  | { type: 'SetReady'; ready: boolean }
  | { type: 'SubmitOrders'; round: number; orders: OrderDto[] }
  | { type: 'CancelOrders'; round: number }
  | { type: 'Ping'; client_time_ms: number };

export type ProtocolErrorCode =
  // Authentication & Controller Permissions
  | 'NotYourUnit'
  | 'InvalidOrderCount'
  | 'PlayerNotAuthenticated'
  | 'UnauthorizedAction'

  // Match & Lobby Lifecycle
  | 'MatchFull'
  | 'LobbyAlreadyStarted'
  | 'NotInLobbyPhase'
  | 'NotInHeroSelectPhase'
  | 'NotInPlanningPhase'
  | 'HeroAlreadySelected'
  | 'InvalidHeroDef'
  | 'HeroSelectLocked'
  | 'PlayerAlreadyConnected'

  // Turn & Timing
  | 'RoundMismatch'
  | 'TurnDeadlineExceeded'
  | 'OrdersAlreadySubmitted'

  // Gameplay Rules
  | 'InsufficientAp'
  | 'InsufficientEnergy'
  | 'AbilityOnCooldown'
  | 'LineOfSightBlocked'
  | 'TargetOutOfRange'
  | 'InvalidTarget'
  | 'TargetDead'
  | 'PathObstructed'

  // Legacy & General codes
  | 'InvalidMessage'
  | 'MatchNotFound'
  | 'NotAuthorized'
  | 'StaleRound'
  | 'InvalidPhase'
  | 'UnitNotOwned'
  | 'UnitDead'
  | 'TimerExpired'
  | 'InternalError'
  | 'InsufficientResources'
  | 'CooldownActive'
  | 'MissingLineOfSight';

export type ServerMessage =
  | { type: 'HelloAck'; player_id: string; reconnect_token: string }
  | { type: 'MatchJoined'; match_id: string; player_id: string; team: number; is_spectator: boolean; snapshot: SnapshotDto }
  | {
      type: 'LobbyUpdated';
      match_id: string;
      phase: MatchPhaseDto;
      players: PlayerLobbyDto[];
      hero_pools: Record<number, HeroDto[]>;
      countdown_ms?: number | null;
    }
  | { type: 'HeroSelected'; player_id: string; team: number; hero_def_id: HeroDefId }
  | { type: 'MatchStarting'; round: number; initial_snapshot: SnapshotDto }
  | { type: 'RoundStarted'; round: number; deadline_unix_ms: number; snapshot: SnapshotDto }
  | { type: 'OrdersAccepted'; round: number }
  | { type: 'OrderRejected'; round: number; error_code: ProtocolErrorCode; reason: string }
  | { type: 'EarlyResolutionTriggered'; round: number; resolution_unix_ms: number }
  | {
      type: 'RoundResolved';
      round: number;
      events: SanitizedGameEvent[];
      snapshot: SnapshotDto;
      state_hash?: string | null;
    }
  | {
      type: 'PlayerConnectionUpdated';
      player_id: string;
      connected: boolean;
      is_ai_controlled: boolean;
    }
  | {
      type: 'MatchEnded';
      winner: number | null;
      snapshot: SnapshotDto;
      state_hash?: string | null;
      total_rounds?: number | null;
    }
  | { type: 'OpponentStatus'; online: boolean }
  | { type: 'Pong'; client_time_ms: number; server_time_ms: number }
  | { type: 'Error'; error_code: ProtocolErrorCode; message: string };
