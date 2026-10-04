import {
  ClientMessage,
  ServerMessage,
  SnapshotDto,
  OrderDto,
  SanitizedGameEvent,
  ProtocolErrorCode,
  MatchPhaseDto,
  PlayerLobbyDto,
  HeroDto,
  HeroDefId,
} from './types';

export type ConnectionState = 'DISCONNECTED' | 'CONNECTING' | 'CONNECTED' | 'RECONNECTING';

export interface NetworkCallbacks {
  onConnectionChange: (state: ConnectionState) => void;
  onMatchJoined: (matchId: string, team: number, snapshot: SnapshotDto) => void;
  onLobbyUpdated?: (
    matchId: string,
    phase: MatchPhaseDto,
    players: PlayerLobbyDto[],
    heroPools: Record<number, HeroDto[]>,
    countdownMs?: number | null
  ) => void;
  onHeroSelected?: (playerId: string, team: number, heroDefId: HeroDefId) => void;
  onMatchStarting?: (round: number, initialSnapshot: SnapshotDto) => void;
  onRoundStarted: (
    round: number,
    deadlineUnixMs: number,
    snapshot: SnapshotDto,
    events?: SanitizedGameEvent[]
  ) => void;
  onOrdersAccepted: (round: number) => void;
  onOrderRejected: (round: number, code: ProtocolErrorCode, reason: string) => void;
  onEarlyResolutionTriggered?: (round: number, resolutionUnixMs: number) => void;
  onRoundResolved: (
    round: number,
    events: SanitizedGameEvent[],
    snapshot: SnapshotDto,
    stateHash?: string | null
  ) => void;
  onPlayerConnectionUpdated?: (playerId: string, connected: boolean, isAiControlled: boolean) => void;
  onMatchEnded: (
    winner: number | null,
    snapshot: SnapshotDto,
    stateHash?: string | null,
    totalRounds?: number | null
  ) => void;
  onOpponentStatus?: (online: boolean) => void;
  onLatency?: (latencyMs: number) => void;
  onPurchaseResolved?: (
    unitId: number,
    itemId: string,
    success: boolean,
    goldRemaining: number,
    error?: ProtocolErrorCode | null
  ) => void;
  onEconomyUpdated?: (
    unitId: number,
    gold: number,
    xp: number,
    level: number,
    items: string[]
  ) => void;
  onLevelUpOccurred?: (
    unitId: number,
    newLevel: number,
    newMaxHp: number,
    newAttackDamage: number,
    newMaxEnergy: number
  ) => void;
  onError: (message: string) => void;
}

export class NetworkBridge {
  private ws: WebSocket | null = null;
  private matchId: string | null = null;
  private playerId: string;
  private reconnectToken: string | null = null;
  private connectionState: ConnectionState = 'DISCONNECTED';
  private callbacks: Partial<NetworkCallbacks> = {};
  private reconnectAttempts = 0;
  private reconnectTimer: number | null = null;
  private pingInterval: number | null = null;
  private queue: ClientMessage[] = [];
  private wsBaseUrl?: string;
  private shouldStopReconnecting = false;

  constructor() {
    const urlParams = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : null;
    const urlPlayerId = urlParams?.get('player_id');

    this.playerId =
      urlPlayerId ??
      sessionStorage.getItem('hb_player_id') ??
      crypto.randomUUID();
    sessionStorage.setItem('hb_player_id', this.playerId);
  }

  setCallbacks(callbacks: Partial<NetworkCallbacks>): void {
    this.callbacks = callbacks;
  }

  getPlayerId(): string {
    return this.playerId;
  }

  getReconnectToken(): string | null {
    return this.reconnectToken;
  }

  getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  getMatchId(): string | null {
    return this.matchId;
  }

  connect(matchId: string, wsBaseUrl?: string): void {
    this.matchId = matchId;
    this.shouldStopReconnecting = false;
    if (wsBaseUrl) this.wsBaseUrl = wsBaseUrl;
    this.cleanupSocket();
    this.setConnectionState('CONNECTING');

    this.reconnectToken = sessionStorage.getItem(`hb_reconnect_token_${matchId}`);

    let url: string;
    if (this.wsBaseUrl) {
      url = `${this.wsBaseUrl}/ws/match/${matchId}?player_id=${encodeURIComponent(this.playerId)}`;
    } else {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      url = `${protocol}//${host}/ws/match/${matchId}?player_id=${encodeURIComponent(this.playerId)}`;
    }

    if (this.reconnectToken) {
      url += `&reconnect_token=${encodeURIComponent(this.reconnectToken)}`;
    }

    try {
      this.ws = new WebSocket(url);
    } catch (e) {
      console.error('[NetworkBridge] WebSocket creation failed:', e);
      this.scheduleReconnect();
      return;
    }

    this.ws.onopen = () => {
      this.reconnectAttempts = 0;
      this.setConnectionState('CONNECTED');
      this.startPing();
      while (this.queue.length > 0) {
        const queuedMsg = this.queue.shift();
        if (queuedMsg) this.send(queuedMsg);
      }
    };

    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as ServerMessage;
        this.handleMessage(msg);
      } catch (err) {
        console.error('[NetworkBridge] Failed to parse message:', err);
      }
    };

    this.ws.onclose = () => {
      this.cleanupSocket();
      this.scheduleReconnect();
    };

    this.ws.onerror = (err) => {
      console.error('[NetworkBridge] WebSocket error:', err);
    };
  }

  selectHero(heroDefId: HeroDefId): void {
    this.send({
      type: 'SelectHero',
      hero_def_id: heroDefId,
    });
  }

  setReady(ready: boolean): void {
    this.send({
      type: 'SetReady',
      ready,
    });
  }

  cancelOrders(round: number): void {
    this.send({
      type: 'CancelOrders',
      round,
    });
  }

  submitOrders(round: number, orders: OrderDto[]): void {
    this.send({
      type: 'SubmitOrders',
      round,
      orders,
    });
  }

  buyItem(itemId: string): void {
    this.send({
      type: 'BuyItem',
      item_id: itemId,
    });
  }

  sendPing(): void {
    this.send({
      type: 'Ping',
      client_time_ms: Date.now(),
    });
  }

  disconnect(): void {
    this.cleanupSocket();
    this.matchId = null;
    this.shouldStopReconnecting = true;
    this.setConnectionState('DISCONNECTED');
  }

  private handleMessage(msg: ServerMessage): void {
    switch (msg.type) {
      case 'HelloAck':
        this.reconnectToken = msg.reconnect_token;
        if (this.matchId) {
          sessionStorage.setItem(`hb_reconnect_token_${this.matchId}`, msg.reconnect_token);
        }
        break;

      case 'MatchJoined':
        this.callbacks.onMatchJoined?.(msg.match_id, msg.team, msg.snapshot);
        break;

      case 'LobbyUpdated':
        this.callbacks.onLobbyUpdated?.(
          msg.match_id,
          msg.phase,
          msg.players,
          msg.hero_pools,
          msg.countdown_ms
        );
        break;

      case 'HeroSelected':
        this.callbacks.onHeroSelected?.(msg.player_id, msg.team, msg.hero_def_id);
        break;

      case 'MatchStarting':
        this.callbacks.onMatchStarting?.(msg.round, msg.initial_snapshot);
        break;

      case 'RoundStarted':
        this.callbacks.onRoundStarted?.(
          msg.round,
          msg.deadline_unix_ms,
          msg.snapshot,
          msg.events
        );
        break;

      case 'OrdersAccepted':
        this.callbacks.onOrdersAccepted?.(msg.round);
        break;

      case 'OrderRejected':
        this.callbacks.onOrderRejected?.(msg.round, msg.error_code, msg.reason);
        break;

      case 'EarlyResolutionTriggered':
        this.callbacks.onEarlyResolutionTriggered?.(msg.round, msg.resolution_unix_ms);
        break;

      case 'RoundResolved':
        this.callbacks.onRoundResolved?.(
          msg.round,
          msg.events,
          msg.snapshot,
          msg.state_hash
        );
        break;

      case 'PlayerConnectionUpdated':
        this.callbacks.onPlayerConnectionUpdated?.(
          msg.player_id,
          msg.connected,
          msg.is_ai_controlled
        );
        break;

      case 'MatchEnded':
        this.callbacks.onMatchEnded?.(
          msg.winner,
          msg.snapshot,
          msg.state_hash,
          msg.total_rounds
        );
        break;

      case 'OpponentStatus':
        this.callbacks.onOpponentStatus?.(msg.online);
        break;

      case 'PurchaseResolved':
        this.callbacks.onPurchaseResolved?.(
          msg.unit_id,
          msg.item_id,
          msg.success,
          msg.gold_remaining,
          msg.error
        );
        break;

      case 'EconomyUpdated':
        this.callbacks.onEconomyUpdated?.(
          msg.unit_id,
          msg.gold,
          msg.xp,
          msg.level,
          msg.items
        );
        break;

      case 'LevelUpOccurred':
        this.callbacks.onLevelUpOccurred?.(
          msg.unit_id,
          msg.new_level,
          msg.new_max_hp,
          msg.new_attack_damage,
          msg.new_max_energy
        );
        break;

      case 'Pong': {
        const now = Date.now();
        const rtt = now - msg.client_time_ms;
        this.callbacks.onLatency?.(rtt);
        break;
      }

      case 'Error':
        if (
          msg.error_code === 'MatchNotFound' ||
          msg.error_code === 'MatchFull' ||
          msg.error_code === 'NotAuthorized'
        ) {
          this.shouldStopReconnecting = true;
        }
        this.callbacks.onError?.(msg.message);
        break;
    }
  }

  private send(msg: ClientMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else if (this.matchId && !this.shouldStopReconnecting && msg.type === 'SubmitOrders') {
      this.queue.push(msg);
    }
  }

  private startPing(): void {
    this.stopPing();
    this.pingInterval = window.setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.sendPing();
      }
    }, 15000);
  }

  private stopPing(): void {
    if (this.pingInterval !== null) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  private scheduleReconnect(): void {
    if (!this.matchId || this.shouldStopReconnecting) {
      this.setConnectionState('DISCONNECTED');
      return;
    }
    this.setConnectionState('RECONNECTING');

    const backoff = Math.min(1000 * Math.pow(1.5, this.reconnectAttempts), 10000);
    this.reconnectAttempts++;

    this.reconnectTimer = window.setTimeout(() => {
      if (this.matchId && !this.shouldStopReconnecting) {
        this.connect(this.matchId, this.wsBaseUrl);
      }
    }, backoff);
  }

  private cleanupSocket(): void {
    this.stopPing();
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.onopen = null;
      this.ws.onmessage = null;
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.close();
      this.ws = null;
    }
  }

  private setConnectionState(state: ConnectionState): void {
    this.connectionState = state;
    this.callbacks.onConnectionChange?.(state);
  }
}
