import { ClientSession, LobbyState } from '../game/client_session';
import { PlayerLobbyDto } from '../game/types';

export class LobbyScreen {
  private overlay: HTMLElement | null = null;
  private session: ClientSession | null = null;
  private isReady = false;

  constructor() {
    this.createDom();
  }

  setSession(session: ClientSession): void {
    this.session = session;
  }

  private createDom(): void {
    let el = document.getElementById('hb-lobby-screen');
    if (!el) {
      el = document.createElement('div');
      el.id = 'hb-lobby-screen';
      el.className = 'hb-lobby-overlay';
      el.style.display = 'none';
      document.body.appendChild(el);
    }
    this.overlay = el;
  }

  show(): void {
    if (this.overlay) {
      this.overlay.style.display = 'flex';
    }
  }

  hide(): void {
    if (this.overlay) {
      this.overlay.style.display = 'none';
    }
  }

  update(lobby: LobbyState): void {
    if (!this.overlay) return;

    if (lobby.phase !== 'Lobby') {
      this.hide();
      return;
    }

    this.show();
    const myPlayerId = this.session?.getNet().getPlayerId();

    const team0Players = lobby.players.filter((p) => p.team === 0);
    const team1Players = lobby.players.filter((p) => p.team === 1);

    const renderSlots = (players: PlayerLobbyDto[], teamName: string, teamClass: string) => {
      let html = `
        <div class="hb-team-column ${teamClass}">
          <div class="hb-team-header ${teamClass}">
            <span>${teamName}</span>
            <span>${players.length}/5 Players</span>
          </div>
      `;

      for (let i = 0; i < 5; i++) {
        const p = players[i];
        if (p) {
          const isSelf = p.player_id === myPlayerId;
          const statusDot = p.connected ? (p.is_ai ? 'ai' : 'online') : 'offline';
          const badgeClass = p.ready ? 'hb-badge-ready' : 'hb-badge-waiting';
          const badgeText = p.ready ? '✓ Ready' : 'Planning';

          html += `
            <div class="hb-slot-card ${isSelf ? 'self' : ''}">
              <div class="hb-slot-left">
                <span class="hb-player-dot ${statusDot}"></span>
                <span class="hb-slot-name">${p.display_name || p.player_id.slice(0, 8)} ${isSelf ? '<strong style="color:#00e676;">[YOU]</strong>' : ''}</span>
              </div>
              <div style="display:flex; align-items:center; gap:8px;">
                ${p.ping_ms ? `<span style="font-size:11px; color:#94a3b8; font-family:monospace;">${p.ping_ms}ms</span>` : ''}
                <span class="hb-slot-badge ${badgeClass}">${badgeText}</span>
              </div>
            </div>
          `;
        } else {
          html += `
            <div class="hb-slot-card empty">
              <span>Empty Slot (AI Backfill)</span>
              <span class="hb-slot-badge hb-badge-ai">🤖 AI</span>
            </div>
          `;
        }
      }

      html += `</div>`;
      return html;
    };

    const myPlayer = lobby.players.find((p) => p.player_id === myPlayerId);
    this.isReady = myPlayer?.ready ?? false;

    this.overlay.innerHTML = `
      <div class="hb-lobby-panel">
        <div class="hb-lobby-header">
          <div class="hb-lobby-title">
            <span>⚔️ 5v5 PRE-GAME LOBBY</span>
          </div>
          <div class="hb-lobby-meta">
            <span>Match: ${lobby.matchId.slice(0, 8)}...</span>
            <button id="btn-copy-lobby-link" class="btn-action-small">📋 Copy Link</button>
          </div>
        </div>

        <div class="hb-lobby-teams-grid">
          ${renderSlots(team0Players, 'Blue Coalition (Team 0)', 'blue')}
          ${renderSlots(team1Players, 'Red Syndicate (Team 1)', 'red')}
        </div>

        <div class="hb-lobby-footer">
          <div style="font-size:13px; color:#94a3b8;">
            All 10 slots filled with human players or dynamic AI fallback.
          </div>
          <div class="hb-lobby-btn-group">
            <button id="btn-lobby-ready" class="btn" style="background:${this.isReady ? '#ff1744' : '#00e676'}; color:#050b14;">
              ${this.isReady ? '❌ Cancel Ready' : '✓ Lock In Ready'}
            </button>
          </div>
        </div>
      </div>
    `;

    // Setup event listeners
    const btnReady = document.getElementById('btn-lobby-ready');
    btnReady?.addEventListener('click', () => {
      this.isReady = !this.isReady;
      this.session?.setReady(this.isReady);
    });

    const btnCopy = document.getElementById('btn-copy-lobby-link');
    btnCopy?.addEventListener('click', () => {
      const url = `${window.location.origin}${window.location.pathname}?match=${lobby.matchId}`;
      navigator.clipboard.writeText(url).then(() => {
        alert('📋 Match invite link copied to clipboard!');
      }).catch(() => {
        navigator.clipboard.writeText(lobby.matchId);
        alert('📋 Match ID copied!');
      });
    });
  }
}
