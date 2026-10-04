import { ConnectionState } from '../game/net';
import { SnapshotDto, RosterEntryDto, UnitDto } from '../game/types';

export class HudController {
  private connPill: HTMLElement | null = null;
  private matchIdEl: HTMLElement | null = null;
  private copyBtn: HTMLElement | null = null;
  private opponentStatusEl: HTMLElement | null = null;
  private reconnectBannerEl: HTMLElement | null = null;
  private rosterSidebarEl: HTMLElement | null = null;
  private enemyBarEl: HTMLElement | null = null;

  private onFocusUnitCallback: ((unitId: number) => void) | null = null;

  constructor() {
    this.connPill = document.getElementById('conn-pill');
    this.matchIdEl = document.getElementById('match-id-display');
    this.copyBtn = document.getElementById('copy-match-btn');
    this.opponentStatusEl = document.getElementById('opponent-status');

    this.createRosterElements();
    this.createReconnectBanner();
    this.setupCopyButton();
  }

  setOnFocusUnit(cb: (unitId: number) => void): void {
    this.onFocusUnitCallback = cb;
  }

  private createRosterElements(): void {
    if (!document.getElementById('hb-roster-sidebar')) {
      const rosterDiv = document.createElement('div');
      rosterDiv.id = 'hb-roster-sidebar';
      rosterDiv.className = 'hb-roster-sidebar';
      document.body.appendChild(rosterDiv);
      this.rosterSidebarEl = rosterDiv;
    } else {
      this.rosterSidebarEl = document.getElementById('hb-roster-sidebar');
    }

    if (!document.getElementById('hb-enemy-bar')) {
      const enemyDiv = document.createElement('div');
      enemyDiv.id = 'hb-enemy-bar';
      enemyDiv.className = 'hb-enemy-bar';
      document.body.appendChild(enemyDiv);
      this.enemyBarEl = enemyDiv;
    } else {
      this.enemyBarEl = document.getElementById('hb-enemy-bar');
    }
  }

  private createReconnectBanner(): void {
    if (!document.getElementById('hb-reconnect-banner')) {
      const banner = document.createElement('div');
      banner.id = 'hb-reconnect-banner';
      banner.style.position = 'fixed';
      banner.style.top = '0';
      banner.style.left = '0';
      banner.style.right = '0';
      banner.style.padding = '8px 16px';
      banner.style.background = 'linear-gradient(90deg, #ff1744, #d50000)';
      banner.style.color = '#fff';
      banner.style.fontSize = '13px';
      banner.style.fontWeight = '800';
      banner.style.textAlign = 'center';
      banner.style.zIndex = '300';
      banner.style.display = 'none';
      banner.style.boxShadow = '0 4px 15px rgba(255, 23, 68, 0.4)';
      banner.innerHTML = `⚠️ NETWORK DISCONNECTED — Reconnecting to match server... Dynamic AI is piloting your hero.`;
      document.body.appendChild(banner);
      this.reconnectBannerEl = banner;
    } else {
      this.reconnectBannerEl = document.getElementById('hb-reconnect-banner');
    }
  }

  setConnectionState(state: ConnectionState): void {
    if (this.connPill) {
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

    if (this.reconnectBannerEl) {
      if (state === 'RECONNECTING') {
        this.reconnectBannerEl.style.display = 'block';
      } else {
        this.reconnectBannerEl.style.display = 'none';
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

  update5v5Rosters(snapshot: SnapshotDto, myPlayerId?: string): void {
    if (!this.rosterSidebarEl || !this.enemyBarEl) return;

    const myTeam = snapshot.player_team ?? 0;
    const is5v5 = snapshot.roster && snapshot.roster.length > 0;

    if (!is5v5) {
      // Legacy 3v3 fallback
      this.rosterSidebarEl.style.display = 'none';
      this.enemyBarEl.style.display = 'none';
      return;
    }

    this.rosterSidebarEl.style.display = 'flex';
    this.enemyBarEl.style.display = 'flex';

    // 1. Render Allied Roster (Team == myTeam)
    const allies = snapshot.roster.filter((r) => r.team === myTeam);
    let allyHtml = `<div style="font-size:11px; font-weight:800; color:#00d2ff; text-transform:uppercase; letter-spacing:0.5px; padding-bottom:2px;">ALLIED TEAM (${allies.length})</div>`;

    for (const ally of allies) {
      const isSelf = Boolean(myPlayerId && ally.player_id === myPlayerId);
      const isControlled = snapshot.controlled_units?.includes(ally.unit_id);
      const currentHp = ally.hp ?? 0;
      const hpRatio = Math.max(0, Math.min(1, currentHp / ally.max_hp));
      const hpColor = hpRatio > 0.5 ? '#00e676' : hpRatio > 0.25 ? '#ffd600' : '#ff1744';

      const statusPipClass = ally.orders_submitted ? 'submitted' : 'planning';
      const statusPipText = ally.orders_submitted ? '✓ READY' : '...';
      const aiBadge = ally.is_ai ? '<span style="color:#38bdf8; font-size:10px;">[AI]</span>' : '';
      const dcBadge = !ally.connected && !ally.is_ai ? '<span style="color:#ff1744; font-size:10px;">[DC]</span>' : '';

      const level = ally.level ?? 1;
      const pipsHtml = [0, 1, 2]
        .map((idx) => {
          const hasItem = ally.items && ally.items.length > idx;
          return `<span class="hb-roster-item-pip ${hasItem ? 'filled' : ''}" title="${hasItem ? ally.items[idx] : 'Empty'}"></span>`;
        })
        .join('');

      allyHtml += `
        <div class="hb-roster-card ${isSelf || isControlled ? 'self' : ''}" data-unit-id="${ally.unit_id}">
          <div class="hb-roster-card-top">
            <span class="hb-roster-hero-name">
              <span>${this.getHeroIcon(ally.hero_def_id)}</span>
              <span>${ally.hero_def_id.toUpperCase()}</span>
              <span class="hb-roster-lvl-tag">L${level}</span>
              ${isSelf ? '<span style="color:#00e676; font-size:10px;">[YOU]</span>' : ''}
              ${aiBadge}
              ${dcBadge}
            </span>
            <div style="display:flex; align-items:center; gap:6px;">
              <div class="hb-roster-item-pips">${pipsHtml}</div>
              <span class="hb-roster-status-pip ${statusPipClass}">${statusPipText}</span>
            </div>
          </div>
          <div class="hb-roster-hp-bar">
            <div class="hb-roster-hp-fill" style="width: ${hpRatio * 100}%; background: ${hpColor};"></div>
          </div>
          <div style="display:flex; justify-content:space-between; font-size:10.5px; font-family:'JetBrains Mono',monospace; color:#94a3b8;">
            <span>HP ${currentHp}/${ally.max_hp}</span>
            <span>#${ally.unit_id}</span>
          </div>
        </div>
      `;
    }
    this.rosterSidebarEl.innerHTML = allyHtml;

    // Attach click listeners to allied cards to focus unit
    const allyCards = this.rosterSidebarEl.querySelectorAll('.hb-roster-card');
    allyCards.forEach((c) => {
      c.addEventListener('click', () => {
        const uid = Number(c.getAttribute('data-unit-id'));
        if (uid && this.onFocusUnitCallback) {
          this.onFocusUnitCallback(uid);
        }
      });
    });

    // 2. Render Spotted Enemies (Team != myTeam)
    const enemies = snapshot.roster.filter((r) => r.team !== myTeam);
    let enemyHtml = `<div style="font-size:11px; font-weight:800; color:#ff3366; text-transform:uppercase; letter-spacing:0.5px; padding-bottom:2px; text-align:right;">SPOTTED ENEMIES (${enemies.length})</div>`;

    for (const enemy of enemies) {
      const isVisibleInFog = enemy.hp !== null && enemy.hp !== undefined;
      const currentHp = isVisibleInFog ? enemy.hp! : enemy.max_hp;
      const hpRatio = isVisibleInFog ? Math.max(0, Math.min(1, currentHp / enemy.max_hp)) : 1;
      const enemyLevel = enemy.level ?? 1;
      const enemyPipsHtml = [0, 1, 2]
        .map((idx) => {
          const hasItem = isVisibleInFog && enemy.items && enemy.items.length > idx;
          return `<span class="hb-roster-item-pip ${hasItem ? 'filled' : ''}" title="${hasItem ? enemy.items[idx] : 'Empty'}"></span>`;
        })
        .join('');

      enemyHtml += `
        <div class="hb-enemy-card ${isVisibleInFog ? '' : 'in-fog'}">
          <div class="hb-enemy-card-top">
            <span class="hb-enemy-hero-name">
              <span>${this.getHeroIcon(enemy.hero_def_id)}</span>
              <span>${enemy.hero_def_id.toUpperCase()}</span>
              <span class="hb-roster-lvl-tag">${isVisibleInFog ? `L${enemyLevel}` : 'L?'}</span>
            </span>
            <div style="display:flex; align-items:center; gap:6px;">
              ${isVisibleInFog ? `<div class="hb-roster-item-pips">${enemyPipsHtml}</div>` : ''}
              <span class="hb-enemy-fog-tag">${isVisibleInFog ? '👀 SIGHTED' : '🌫️ IN FOG'}</span>
            </div>
          </div>
          <div class="hb-roster-hp-bar">
            <div class="hb-roster-hp-fill" style="width: ${isVisibleInFog ? hpRatio * 100 : 100}%; background: ${isVisibleInFog ? '#ff3366' : '#475569'};"></div>
          </div>
          <div style="display:flex; justify-content:space-between; font-size:10.5px; font-family:'JetBrains Mono',monospace; color:#94a3b8;">
            <span>${isVisibleInFog ? `HP ${currentHp}/${enemy.max_hp}` : `HP ??? / ${enemy.max_hp}`}</span>
            <span>#${enemy.unit_id}</span>
          </div>
        </div>
      `;
    }
    this.enemyBarEl.innerHTML = enemyHtml;
  }

  private getHeroIcon(defId: string): string {
    switch (defId.toLowerCase()) {
      case 'vanguard': return '🛡️';
      case 'ranger': return '🏹';
      case 'warden': return '⚕️';
      case 'sniper': return '🎯';
      case 'berserker': return '🪓';
      default: return '👤';
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
