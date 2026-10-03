import { ConnectionState } from '../game/net';

export class HudController {
  private connPill: HTMLElement | null = null;
  private matchIdEl: HTMLElement | null = null;
  private copyBtn: HTMLElement | null = null;
  private opponentStatusEl: HTMLElement | null = null;

  constructor() {
    this.connPill = document.getElementById('conn-pill');
    this.matchIdEl = document.getElementById('match-id-display');
    this.copyBtn = document.getElementById('copy-match-btn');
    this.opponentStatusEl = document.getElementById('opponent-status');
    this.setupCopyButton();
  }

  setConnectionState(state: ConnectionState): void {
    if (!this.connPill) return;
    this.connPill.className = `conn-pill ${state.toLowerCase()}`;
    const label = this.connPill.querySelector('.conn-text');
    if (label) {
      switch (state) {
        case 'CONNECTED':
          label.textContent = 'ONLINE';
          break;
        case 'CONNECTING':
          label.textContent = 'CONNECTING...';
          break;
        case 'RECONNECTING':
          label.textContent = 'RECONNECTING...';
          break;
        case 'DISCONNECTED':
          label.textContent = 'OFFLINE';
          break;
      }
    }
  }

  setMatchInfo(matchId: string | null, isPvAI: boolean): void {
    if (this.matchIdEl) {
      if (matchId) {
        this.matchIdEl.textContent = `Match: ${matchId.slice(0, 8)}...`;
        this.matchIdEl.setAttribute('data-full-id', matchId);
        this.matchIdEl.style.display = 'inline-block';
      } else {
        this.matchIdEl.textContent = '';
        this.matchIdEl.style.display = 'none';
      }
    }
    if (this.copyBtn) {
      this.copyBtn.style.display = matchId && !isPvAI ? 'inline-flex' : 'none';
    }
  }

  setOpponentStatus(status: 'waiting' | 'ready' | 'offline' | 'ai' | 'hidden'): void {
    if (!this.opponentStatusEl) return;
    if (status === 'hidden') {
      this.opponentStatusEl.style.display = 'none';
      return;
    }
    this.opponentStatusEl.style.display = 'block';
    switch (status) {
      case 'waiting':
        this.opponentStatusEl.textContent = 'Waiting for Opponent...';
        this.opponentStatusEl.style.color = '#ffea00';
        break;
      case 'ready':
        this.opponentStatusEl.textContent = 'Opponent Connected';
        this.opponentStatusEl.style.color = '#00e676';
        break;
      case 'offline':
        this.opponentStatusEl.textContent = 'Opponent Offline (AI Auto-play active)';
        this.opponentStatusEl.style.color = '#ff1744';
        break;
      case 'ai':
        this.opponentStatusEl.textContent = 'Opponent: Tactical Server AI';
        this.opponentStatusEl.style.color = '#00d2ff';
        break;
    }
  }

  showToast(message: string): void {
    let toast = document.getElementById('hud-toast');
    if (!toast) {
      toast = document.createElement('div');
      toast.id = 'hud-toast';
      document.body.appendChild(toast);
    }
    toast.textContent = message;
    toast.className = 'show';
    setTimeout(() => {
      toast?.classList.remove('show');
    }, 2500);
  }

  private setupCopyButton(): void {
    if (!this.copyBtn) return;
    this.copyBtn.addEventListener('click', () => {
      const matchId = this.matchIdEl?.getAttribute('data-full-id');
      if (matchId) {
        const url = `${window.location.origin}${window.location.pathname}?match=${matchId}`;
        navigator.clipboard.writeText(url).then(() => {
          this.showToast('📋 Match link copied to clipboard!');
        }).catch(() => {
          navigator.clipboard.writeText(matchId);
          this.showToast('📋 Match ID copied to clipboard!');
        });
      }
    });
  }
}
