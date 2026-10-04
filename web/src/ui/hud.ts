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
      const isDead = ally.life_state === 'dead_awaiting_respawn' || (ally.hp === 0 && ally.hp !== null);
      const currentHp = ally.hp ?? 0;
      const hpRatio = isDead ? 0 : Math.max(0, Math.min(1, currentHp / ally.max_hp));
      const hpColor = isDead ? '#64748b' : hpRatio > 0.5 ? '#00e676' : hpRatio > 0.25 ? '#ffd600' : '#ff1744';

      const statusPipClass = isDead ? 'dead' : ally.orders_submitted ? 'submitted' : 'planning';
      const statusPipText = isDead
        ? `💀 ${ally.respawn_rounds ?? 0}R`
        : ally.orders_submitted
        ? '✓ READY'
        : '...';
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
        <div class="hb-roster-card ${isSelf || isControlled ? 'self' : ''} ${isDead ? 'dead-card' : ''}" data-unit-id="${ally.unit_id}">
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
            <span>${isDead ? `💀 RESPAWNING (${ally.respawn_rounds ?? 0}r)` : `HP ${currentHp}/${ally.max_hp}`}</span>
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
      const isEnemyDead = enemy.life_state === 'dead_awaiting_respawn';
      const isVisibleInFog = enemy.hp !== null && enemy.hp !== undefined;
      const currentHp = isEnemyDead ? 0 : isVisibleInFog ? enemy.hp! : enemy.max_hp;
      const hpRatio = isEnemyDead ? 0 : isVisibleInFog ? Math.max(0, Math.min(1, currentHp / enemy.max_hp)) : 1;
      const enemyLevel = enemy.level ?? 1;
      const enemyPipsHtml = [0, 1, 2]
        .map((idx) => {
          const hasItem = isVisibleInFog && enemy.items && enemy.items.length > idx;
          return `<span class="hb-roster-item-pip ${hasItem ? 'filled' : ''}" title="${hasItem ? enemy.items[idx] : 'Empty'}"></span>`;
        })
        .join('');

      const enemyTagText = isEnemyDead
        ? `💀 ${enemy.respawn_rounds ?? 0}R`
        : isVisibleInFog
        ? '👀 SIGHTED'
        : '🌫️ IN FOG';

      enemyHtml += `
        <div class="hb-enemy-card ${isEnemyDead ? 'dead-card' : isVisibleInFog ? '' : 'in-fog'}">
          <div class="hb-enemy-card-top">
            <span class="hb-enemy-hero-name">
              <span>${this.getHeroIcon(enemy.hero_def_id)}</span>
              <span>${enemy.hero_def_id.toUpperCase()}</span>
              <span class="hb-roster-lvl-tag">${isVisibleInFog ? `L${enemyLevel}` : 'L?'}</span>
            </span>
            <div style="display:flex; align-items:center; gap:6px;">
              ${isVisibleInFog ? `<div class="hb-roster-item-pips">${enemyPipsHtml}</div>` : ''}
              <span class="hb-enemy-fog-tag ${isEnemyDead ? 'dead' : ''}">${enemyTagText}</span>
            </div>
          </div>
          <div class="hb-roster-hp-bar">
            <div class="hb-roster-hp-fill" style="width: ${isVisibleInFog ? hpRatio * 100 : 100}%; background: ${isEnemyDead ? '#475569' : isVisibleInFog ? '#ff3366' : '#475569'};"></div>
          </div>
          <div style="display:flex; justify-content:space-between; font-size:10.5px; font-family:'JetBrains Mono',monospace; color:#94a3b8;">
            <span>${isEnemyDead ? `💀 RESPAWNING (${enemy.respawn_rounds ?? 0}r)` : isVisibleInFog ? `HP ${currentHp}/${enemy.max_hp}` : `HP ??? / ${enemy.max_hp}`}</span>
            <span>#${enemy.unit_id}</span>
          </div>
        </div>
      `;
    }
    this.enemyBarEl.innerHTML = enemyHtml;
  }

  updateMacroBar(snapshot: SnapshotDto): void {
    let macroBar = document.getElementById('hb-macro-bar');
    if (!macroBar) {
      macroBar = document.createElement('div');
      macroBar.id = 'hb-macro-bar';
      macroBar.className = 'hb-macro-bar';
      const hudEl = document.getElementById('hud');
      if (hudEl && hudEl.parentNode) {
        hudEl.parentNode.insertBefore(macroBar, hudEl.nextSibling);
      } else {
        document.body.appendChild(macroBar);
      }
    }

    const myTeam = snapshot.player_team ?? 0;
    const coreHp = snapshot.core_hp;
    const team0Hp = coreHp ? coreHp[0] : null;
    const team1Hp = coreHp ? coreHp[1] : null;

    const t0Cur = team0Hp ? team0Hp[0] : 700;
    const t0Max = team0Hp ? team0Hp[1] : 700;
    const t1Cur = team1Hp ? team1Hp[0] : 700;
    const t1Max = team1Hp ? team1Hp[1] : 700;

    const obj = snapshot.objective;
    let objHtml = '';
    if (obj) {
      if (obj.is_alive) {
        objHtml = `
          <div class="hb-macro-obj alive" title="The Ancient Vault: Destroy to grant your living team +50 Gold, +40 XP, and +5 Attack Damage for 5 rounds">
            <span class="hb-macro-icon">🏛️</span>
            <span class="hb-macro-label">VAULT</span>
            <span class="hb-macro-hp">${obj.hp} / ${obj.max_hp}</span>
          </div>
        `;
      } else {
        objHtml = `
          <div class="hb-macro-obj destroyed" title="Destroyed Vault">
            <span class="hb-macro-icon">🏛️</span>
            <span class="hb-macro-label" style="color: #f59e0b;">VAULT SECURED</span>
          </div>
        `;
      }
    }

    const myCoreCur = myTeam === 0 ? t0Cur : t1Cur;
    const myCoreMax = myTeam === 0 ? t0Max : t1Max;
    const enemyCoreCur = myTeam === 0 ? t1Cur : t0Cur;
    const enemyCoreMax = myTeam === 0 ? t1Max : t0Max;

    macroBar.innerHTML = `
      <div class="hb-macro-core ally" title="Allied Core">
        <span class="hb-macro-icon">💎</span>
        <span class="hb-macro-label">ALLIED CORE</span>
        <div class="hb-macro-bar-fill-wrap">
          <div class="hb-macro-bar-fill ally-fill" style="width: ${(myCoreCur / myCoreMax) * 100}%;"></div>
        </div>
        <span class="hb-macro-hp">${myCoreCur} / ${myCoreMax}</span>
      </div>

      ${objHtml}

      <div class="hb-macro-core enemy" title="Enemy Core">
        <span class="hb-macro-icon">💎</span>
        <span class="hb-macro-label">ENEMY CORE</span>
        <div class="hb-macro-bar-fill-wrap">
          <div class="hb-macro-bar-fill enemy-fill" style="width: ${(enemyCoreCur / enemyCoreMax) * 100}%;"></div>
        </div>
        <span class="hb-macro-hp">${enemyCoreCur} / ${enemyCoreMax}</span>
      </div>
    `;
  }

  setHeroDefeated(defeated: boolean, roundsLeft: number): void {
    let banner = document.getElementById('hb-respawn-banner');
    if (!banner) {
      banner = document.createElement('div');
      banner.id = 'hb-respawn-banner';
      banner.className = 'hb-respawn-banner';
      document.body.appendChild(banner);
    }

    if (defeated) {
      document.body.classList.add('hero-defeated');
      banner.innerHTML = `
        <div class="hb-respawn-card">
          <div class="hb-respawn-header">💀 HERO FALLEN</div>
          <div class="hb-respawn-countdown">RESPAWNING AT BASE IN ${roundsLeft} ROUND${roundsLeft === 1 ? '' : 'S'}</div>
          <div class="hb-respawn-sub">Progression and items preserved. Spectate freely.</div>
        </div>
      `;
      banner.style.display = 'flex';
    } else {
      document.body.classList.remove('hero-defeated');
      banner.style.display = 'none';
    }
  }

  showVictoryCelebrationModal(params: {
    won: boolean;
    isDraw: boolean;
    round: number;
    stats?: {
      alliedCoreHp: number;
      enemyCoreHp: number;
      vaultSecured: boolean;
      totalGold: number;
      items: string[];
    };
    onReturnToLobby: () => void;
  }): void {
    let modal = document.getElementById('hb-celebration-modal');
    if (!modal) {
      modal = document.createElement('div');
      modal.id = 'hb-celebration-modal';
      modal.className = 'hb-celebration-modal';
      document.body.appendChild(modal);
    }

    const { won, isDraw, round, stats, onReturnToLobby } = params;

    let headerText = '';
    let headerClass = '';
    let subtitleText = '';

    if (won) {
      headerText = 'VICTORY — ENEMY CORE OBLITERATED';
      headerClass = 'victory';
      subtitleText = `Decisive victory achieved in Round ${round}`;
    } else if (isDraw) {
      headerText = 'DRAW — MUTUAL CORE DESTRUCTION';
      headerClass = 'draw';
      subtitleText = `Battle concluded in Round ${round}`;
    } else {
      headerText = 'DEFEAT — ALLIED CORE COLLAPSED';
      headerClass = 'defeat';
      subtitleText = `Your Core was destroyed in Round ${round}`;
    }

    const itemsHtml = stats?.items && stats.items.length > 0
      ? stats.items.map(i => `<span class="hb-celeb-item-tag">${this.formatItemName(i)}</span>`).join('')
      : '<span style="color:#64748b;">No items equipped</span>';

    modal.innerHTML = `
      <div class="hb-celebration-card ${headerClass}">
        <div class="hb-celebration-crown">${won ? '🏆' : isDraw ? '⚖️' : '💀'}</div>
        <h1 class="hb-celebration-title ${headerClass}">${headerText}</h1>
        <div class="hb-celebration-sub">${subtitleText}</div>

        <div class="hb-celebration-stats-grid">
          <div class="hb-celeb-stat">
            <span class="label">Rounds Fought</span>
            <span class="val">${round}</span>
          </div>
          <div class="hb-celeb-stat">
            <span class="label">Allied Core HP</span>
            <span class="val" style="color:#38bdf8;">${stats?.alliedCoreHp ?? '-'} / 700</span>
          </div>
          <div class="hb-celeb-stat">
            <span class="label">Enemy Core HP</span>
            <span class="val" style="color:#fb7185;">${stats?.enemyCoreHp ?? '-'} / 700</span>
          </div>
          <div class="hb-celeb-stat">
            <span class="label">Ancient Vault</span>
            <span class="val" style="color:#f59e0b;">${stats?.vaultSecured ? 'SECURED (+50G)' : 'UNCLAIMED'}</span>
          </div>
          <div class="hb-celeb-stat">
            <span class="label">Total Gold</span>
            <span class="val" style="color:#ffd700;">🪙 ${stats?.totalGold ?? 50} G</span>
          </div>
          <div class="hb-celeb-stat full-width">
            <span class="label">Final Equipment Build</span>
            <div class="hb-celeb-items-list">${itemsHtml}</div>
          </div>
        </div>

        <button id="hb-btn-return-lobby" class="hb-celebration-btn ${headerClass}">
          RETURN TO LOBBY
        </button>
      </div>
    `;

    modal.style.display = 'flex';
    document.getElementById('hb-btn-return-lobby')?.addEventListener('click', () => {
      modal!.style.display = 'none';
      onReturnToLobby();
    });
  }

  private formatItemName(id: string): string {
    switch (id) {
      case 'longblade': return '⚔️ Longblade';
      case 'plate_armor': return '🛡️ Plate Armor';
      case 'scout_lens': return '👁️ Scout Lens';
      case 'focus_charm': return '🔮 Focus Charm';
      default: return id;
    }
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
