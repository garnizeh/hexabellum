import {
  ClientMessage,
  ServerMessage,
  SnapshotDto,
  OrderDto,
  SanitizedGameEvent,
  ProtocolErrorCode,
} from './types';

export type ConnectionState = 'DISCONNECTED' | 'CONNECTING' | 'CONNECTED' | 'RECONNECTING';

export interface NetworkCallbacks {
  onConnectionChange: (state: ConnectionState) => void;
  onMatchJoined: (team: number, snapshot: SnapshotDto) => void;
  onRoundStarted: (round: number, deadlineUnixMs: number, snapshot: SnapshotDto) => void;
  onOrdersAccepted: (round: number) => void;
  onOrderRejected: (round: number, code: ProtocolErrorCode, reason: string) => void;
  onRoundResolved: (round: number, events: SanitizedGameEvent[], snapshot: SnapshotDto) => void;
  onMatchEnded: (winner: number | null, snapshot: SnapshotDto) => void;
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
  private wsBaseUrl?: string;

  constructor() {
    this.playerId = localStorage.getItem('hb_player_id') ?? crypto.randomUUID();
    localStorage.setItem('hb_player_id', this.playerId);
    this.reconnectToken = localStorage.getItem('hb_reconnect_token');
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
    if (wsBaseUrl) this.wsBaseUrl = wsBaseUrl;
    this.cleanupSocket();
    this.setConnectionState('CONNECTING');

    let url: string;
    if (this.wsBaseUrl) {
      url = `${this.wsBaseUrl}/ws/match/${matchId}?player_id=${this.playerId}`;
    } else {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      url = `${protocol}//${host}/ws/match/${matchId}?player_id=${this.playerId}`;
    }

    if (this.reconnectToken) {
      url += `&reconnect_token=${this.reconnectToken}`;
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

  submitOrders(round: number, orders: OrderDto[]): void {
    this.send({
      type: 'SubmitOrders',
      round,
      orders,
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
    this.setConnectionState('DISCONNECTED');
  }

  private handleMessage(msg: ServerMessage): void {
    switch (msg.type) {
      case 'HelloAck':
        this.reconnectToken = msg.reconnect_token;
        localStorage.setItem('hb_reconnect_token', msg.reconnect_token);
        break;

      case 'MatchJoined':
        this.callbacks.onMatchJoined?.(msg.team, msg.snapshot);
        break;

      case 'RoundStarted':
        this.callbacks.onRoundStarted?.(msg.round, msg.deadline_unix_ms, msg.snapshot);
        break;

      case 'OrdersAccepted':
        this.callbacks.onOrdersAccepted?.(msg.round);
        break;

      case 'OrderRejected':
        this.callbacks.onOrderRejected?.(msg.round, msg.error_code, msg.reason);
        break;

      case 'RoundResolved':
        this.callbacks.onRoundResolved?.(msg.round, msg.events, msg.snapshot);
        break;

      case 'MatchEnded':
        this.callbacks.onMatchEnded?.(msg.winner, msg.snapshot);
        break;

      case 'Error':
        this.callbacks.onError?.(msg.message);
        break;
    }
  }

  private send(msg: ClientMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }

  private scheduleReconnect(): void {
    if (!this.matchId) return;
    this.setConnectionState('RECONNECTING');

    const backoff = Math.min(1000 * Math.pow(1.5, this.reconnectAttempts), 10000);
    this.reconnectAttempts++;

    this.reconnectTimer = window.setTimeout(() => {
      if (this.matchId) {
        this.connect(this.matchId, this.wsBaseUrl);
      }
    }, backoff);
  }

  private cleanupSocket(): void {
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
