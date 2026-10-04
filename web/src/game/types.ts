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

export type ItemDefId = string;

export type StatKind =
  | 'AttackDamage'
  | 'MaxHealth'
  | 'MaxHp'
  | 'VisionRange'
  | 'EnergyRegen'
  | 'MaxEnergy'
  | 'MaxAp'
  | 'Initiative';

export interface StatModifierDto {
  stat: StatKind;
  value: number;
}

export interface ItemDto {
  id: ItemDefId;
  name: string;
  cost: number;
  description: string;
  modifiers: StatModifierDto[];
}

export interface HeroEconomyDto {
  unit_id: number;
  gold: number;
  xp: number;
  level: number;
  items: ItemDefId[];
}

export type RewardReason =
  | 'PassiveIncome'
  | 'HeroKill'
  | 'MinionKill'
  | 'NeutralKill'
  | 'TowerDestroyed'
  | 'SpawnerDestroyed'
  | 'TowerKill'
  | 'SpawnerKill'
  | 'NeutralCamp';

export type LifeStateDto =
  | 'alive'
  | 'dead_awaiting_respawn'
  | 'permanently_removed';

export interface BaseZoneDto {
  team: number;
  center: HexDto;
  radius: number;
}

export interface ObjectiveStatusDto {
  unit_id: number;
  pos: HexDto;
  hp: number;
  max_hp: number;
  is_alive: boolean;
}

export type ShopDisabledReasonDto =
  | 'not_planning_phase'
  | 'hero_dead'
  | 'outside_base_zone'
  | 'inventory_full'
  | 'insufficient_gold';

export interface UnitDto {
  id: number;
  kind: string; // "Hero" | "Minion" | "Tower" | "Spawner" | "NeutralGuardian" | "Core" | "Objective"
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
  gold?: number | null;
  xp?: number | null;
  level: number;
  items: ItemDefId[];
  life_state?: LifeStateDto;
  respawn_rounds?: number | null;
  death_pos?: HexDto | null;
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
  level: number;
  items: ItemDefId[];
  life_state?: LifeStateDto | null;
  respawn_rounds?: number | null;
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
  controlled_hero_economy?: HeroEconomyDto | null;
  allied_hero_economy: HeroEconomyDto[];
  shop_catalog: ItemDto[];
  can_shop: boolean;
  victory_mode?: string | null;
  base_zones?: BaseZoneDto[];
  shop_disabled_reason?: ShopDisabledReasonDto | null;
  core_hp?: Record<number, [number, number]>;
  objective?: ObjectiveStatusDto | null;
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
  | { type: 'RewardGranted'; unit_id: number; gold: number; xp: number; reason: RewardReason }
  | {
      type: 'LevelUp';
      unit_id: number;
      new_level: number;
      new_max_hp: number;
      new_attack_damage: number;
      new_max_energy: number;
    }
  | { type: 'HeroDied'; unit_id: number; killed_by: number; respawn_rounds: number }
  | { type: 'HeroRespawned'; unit_id: number; team: number; pos: HexDto }
  | { type: 'BaseRegenerationApplied'; unit_id: number; team: number; amount: number; new_hp: number }
  | {
      type: 'ObjectiveDestroyed';
      objective_id: number;
      destroyer_team: number;
      last_attacker_id: number;
      gold_awarded_per_hero: number;
      xp_awarded_per_hero: number;
      affected_heroes: number[];
    }
  | { type: 'CoreDestroyed'; core_id: number; team: number; destroyed_by: number }
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
  | { type: 'BuyItem'; item_id: ItemDefId }
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
  | 'MissingLineOfSight'

  // Phase 6 Economic Error Codes
  | 'InsufficientGold'
  | 'InventoryFull'
  | 'ItemAlreadyOwned'
  | 'NoSuchItem'
  | 'CannotShopInPhase'
  | 'NotAHero'

  // Phase 7 Macro Error Codes
  | 'CannotShopOutsideBase'
  | 'HeroDeadAwaitingRespawn'
  | 'CannotOrderDeadHero'
  | 'TargetUntargetable'
  | 'CoreCannotBeRepaired';

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
      type: 'PurchaseResolved';
      unit_id: number;
      item_id: ItemDefId;
      success: boolean;
      gold_remaining: number;
      error?: ProtocolErrorCode | null;
    }
  | {
      type: 'EconomyUpdated';
      unit_id: number;
      gold: number;
      xp: number;
      level: number;
      items: ItemDefId[];
    }
  | {
      type: 'LevelUpOccurred';
      unit_id: number;
      new_level: number;
      new_max_hp: number;
      new_attack_damage: number;
      new_max_energy: number;
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
