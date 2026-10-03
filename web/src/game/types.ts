export interface HexDto {
  q: number;
  r: number;
}

export type HexCoord = HexDto;

export interface UnitDto {
  id: number;
  kind: string; // "Hero" | "Minion" | "Tower" | "SpawnerTower" | "Neutral"
  team: number;
  pos: HexDto;
  hp: number;
  max_hp: number;
  ap: number;
  max_ap: number;
  initiative: number;
  attack_damage: number;
  attack_range: number;
  vision_range: number;
  is_stationary: boolean;
}

export interface MapDto {
  radius: number;
  walkable: HexDto[];
  obstacles: HexDto[];
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
}

export type ActionDto =
  | { type: 'Wait' }
  | { type: 'Attack'; target_id: number };

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
  | { type: 'UnitDied'; unit_id: number; unit_kind: string; killed_by: number }
  | { type: 'UnitWaited'; unit_id: number }
  | { type: 'RoundEnded'; round: number }
  | { type: 'MatchEnded'; winner: number | null };

export type ClientMessage =
  | { type: 'Hello'; player_id: string; reconnect_token?: string | null }
  | { type: 'JoinMatch'; match_id: string }
  | { type: 'SubmitOrders'; round: number; orders: OrderDto[] }
  | { type: 'Ping'; client_time_ms: number };

export type ProtocolErrorCode =
  | 'InvalidMessage'
  | 'MatchNotFound'
  | 'MatchFull'
  | 'NotAuthorized'
  | 'StaleRound'
  | 'InvalidPhase'
  | 'UnitNotOwned'
  | 'UnitDead'
  | 'InvalidTarget'
  | 'TimerExpired'
  | 'InternalError';

export type ServerMessage =
  | { type: 'HelloAck'; player_id: string; reconnect_token: string }
  | { type: 'MatchJoined'; match_id: string; player_id: string; team: number; is_spectator: boolean; snapshot: SnapshotDto }
  | { type: 'RoundStarted'; round: number; deadline_unix_ms: number; snapshot: SnapshotDto }
  | { type: 'OrdersAccepted'; round: number }
  | { type: 'OrderRejected'; round: number; error_code: ProtocolErrorCode; reason: string }
  | { type: 'RoundResolved'; round: number; events: SanitizedGameEvent[]; snapshot: SnapshotDto }
  | { type: 'MatchEnded'; winner: number | null; snapshot: SnapshotDto }
  | { type: 'OpponentStatus'; online: boolean }
  | { type: 'Pong'; client_time_ms: number; server_time_ms: number }
  | { type: 'Error'; error_code: ProtocolErrorCode; message: string };
