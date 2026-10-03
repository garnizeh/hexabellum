import { ClientSession, LobbyState } from '../game/client_session';
import { HeroDefId } from '../game/types';

interface HeroSpec {
  id: HeroDefId;
  name: string;
  icon: string;
  role: string;
  stats: {
    hp: number;
    ap: number;
    dmg: number;
    rng: number;
    vis: number;
    eng: number;
  };
  ability: {
    name: string;
    cost: string;
    desc: string;
  };
}

const HEROES: HeroSpec[] = [
  {
    id: 'vanguard',
    name: 'Vanguard',
    icon: '🛡️',
    role: 'Frontline Bruiser',
    stats: { hp: 140, ap: 3, dmg: 20, rng: 1, vis: 3, eng: 5 },
    ability: {
      name: 'Cleave',
      cost: '⚡3 Energy · 1 AP',
      desc: 'Cleaves all adjacent enemies in a 1-hex radial sweep for 30 physical damage.',
    },
  },
  {
    id: 'ranger',
    name: 'Ranger',
    icon: '🏹',
    role: 'Ranged Marksman',
    stats: { hp: 90, ap: 3, dmg: 16, rng: 2, vis: 4, eng: 5 },
    ability: {
      name: 'Bolt',
      cost: '⚡2 Energy · 1 AP',
      desc: 'Fires an energy-infused projectile dealing 25 damage (Range 3, requires LOS).',
    },
  },
  {
    id: 'warden',
    name: 'Warden',
    icon: '⚕️',
    role: 'Combat Medic',
    stats: { hp: 100, ap: 3, dmg: 12, rng: 1, vis: 3, eng: 6 },
    ability: {
      name: 'Mend',
      cost: '⚡2 Energy · 1 AP',
      desc: 'Channels revitalizing energy to heal an allied unit for +30 HP (Range 2).',
    },
  },
  {
    id: 'sniper',
    name: 'Sniper',
    icon: '🎯',
    role: 'Artillery Marksman',
    stats: { hp: 80, ap: 3, dmg: 14, rng: 3, vis: 5, eng: 5 },
    ability: {
      name: 'Longshot',
      cost: '⚡3 Energy · 1 AP',
      desc: 'High-caliber artillery round dealing 30 damage (Range 4, Min Range 2, CD 3, requires LOS).',
    },
  },
  {
    id: 'berserker',
    name: 'Berserker',
    icon: '🪓',
    role: 'Melee Rage Bruiser',
    stats: { hp: 120, ap: 3, dmg: 20, rng: 1, vis: 3, eng: 5 },
    ability: {
      name: 'Fury',
      cost: '⚡2 Energy · 1 AP',
      desc: 'Ignites furious rage, granting +8 Attack Damage buff for 2 rounds (CD 3).',
    },
  },
];

export class HeroSelectScreen {
  private overlay: HTMLElement | null = null;
  private session: ClientSession | null = null;
  private selectedHeroId: HeroDefId | null = null;
  private countdownTimer: number | null = null;
  private remainingSeconds: number = 20;

  constructor() {
    this.createDom();
  }

  setSession(session: ClientSession): void {
    this.session = session;
  }

  private createDom(): void {
    let el = document.getElementById('hb-hero-select-screen');
    if (!el) {
      el = document.createElement('div');
      el.id = 'hb-hero-select-screen';
      el.className = 'hb-draft-overlay';
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
    if (this.countdownTimer !== null) {
      clearInterval(this.countdownTimer);
      this.countdownTimer = null;
    }
  }

  update(lobby: LobbyState): void {
    if (!this.overlay) return;

    if (lobby.phase !== 'HeroSelect') {
      this.hide();
      return;
    }

    this.show();
    const myPlayerId = this.session?.getNet().getPlayerId();
    const myPlayer = lobby.players.find((p) => p.player_id === myPlayerId);
    const myTeam = myPlayer?.team ?? 0;

    // Check heroes picked by allies on my team to enforce team uniqueness
    const alliedPicks = new Map<HeroDefId, string>();
    for (const p of lobby.players) {
      if (p.team === myTeam && p.hero_def_id && p.player_id !== myPlayerId) {
        alliedPicks.set(p.hero_def_id, p.display_name || p.player_id.slice(0, 6));
      }
    }

    // Retain or sync my selected hero
    if (myPlayer?.hero_def_id) {
      this.selectedHeroId = myPlayer.hero_def_id;
    }

    // Compute remaining countdown seconds
    if (lobby.countdownMs !== undefined && lobby.countdownMs !== null) {
      this.remainingSeconds = Math.max(0, Math.ceil(lobby.countdownMs / 1000));
    }

    const renderHeroCard = (h: HeroSpec) => {
      const isSelected = this.selectedHeroId === h.id;
      const lockedBy = alliedPicks.get(h.id);
      const isLocked = Boolean(lockedBy);

      return `
        <div class="hb-hero-card ${isSelected ? 'selected' : ''} ${isLocked ? 'locked' : ''}" data-hero-id="${h.id}">
          ${isLocked ? `<div class="hb-locked-ribbon">LOCKED</div>` : ''}
          <div class="hb-hero-card-icon">${h.icon}</div>
          <div class="hb-hero-card-name">${h.name}</div>
          <div class="hb-hero-card-role">${h.role}</div>

          <div class="hb-hero-stats">
            <div class="hb-hero-stat-row"><span>HP</span><span>${h.stats.hp}</span></div>
            <div class="hb-hero-stat-row"><span>AP</span><span>${h.stats.ap}</span></div>
            <div class="hb-hero-stat-row"><span>ATK</span><span>${h.stats.dmg}</span></div>
            <div class="hb-hero-stat-row"><span>RNG</span><span>${h.stats.rng}</span></div>
            <div class="hb-hero-stat-row"><span>VIS</span><span>${h.stats.vis}</span></div>
            <div class="hb-hero-stat-row"><span>ENG</span><span>${h.stats.eng}</span></div>
          </div>

          <div class="hb-hero-ability-preview">
            <div class="hb-hero-ability-title">${h.ability.name} <span style="font-size:10px; color:#94a3b8;">(${h.ability.cost})</span></div>
            <div>${h.ability.desc}</div>
          </div>

          ${
            isLocked
              ? `<div style="font-size:11px; color:#ff1744; text-align:center; font-weight:700;">Locked by ${lockedBy}</div>`
              : `<button class="btn btn-select-hero" data-hero-id="${h.id}" style="width:100%; margin-top:4px; padding:6px 0; font-size:12px; background:${isSelected ? '#00e676' : 'rgba(0,210,255,0.2)'}; color:${isSelected ? '#050b14' : '#00d2ff'};">
                  ${isSelected ? '✓ Selected' : 'Choose Hero'}
                </button>`
          }
        </div>
      `;
    };

    this.overlay.innerHTML = `
      <div class="hb-draft-panel">
        <div class="hb-draft-header">
          <div>
            <h2 style="font-size:24px; font-weight:900; color:#00d2ff; letter-spacing:0.8px;">HERO DRAFT — SELECT YOUR AVATAR</h2>
            <div style="font-size:13px; color:#94a3b8; margin-top:2px;">
              Team ${myTeam === 0 ? 'Blue Coalition' : 'Red Syndicate'} · Unique picks enforced per team
            </div>
          </div>
          <div class="hb-draft-timer-box">
            <span style="font-size:13px; font-weight:800; color:#cbd5e1; text-transform:uppercase;">Time Remaining:</span>
            <div id="hb-draft-countdown" class="hb-countdown-badge">${this.remainingSeconds}s</div>
          </div>
        </div>

        <div class="hb-heroes-grid">
          ${HEROES.map(renderHeroCard).join('')}
        </div>

        <div style="display:flex; justify-content:space-between; align-items:center; border-top:1px solid rgba(0,210,255,0.2); padding-top:14px;">
          <div style="font-size:12px; color:#94a3b8;">
            ⚠️ If no hero is locked in before time expires, the server will auto-assign a random available hero.
          </div>
          <div>
            <button id="btn-lock-in-hero" class="btn" style="background:${this.selectedHeroId ? 'linear-gradient(135deg, #00e676, #00b359)' : '#252e3d'}; color:#050b14; padding:10px 28px; font-size:15px;" ${this.selectedHeroId ? '' : 'disabled'}>
              ${this.selectedHeroId ? `Lock In ${this.selectedHeroId.toUpperCase()} ⚔️` : 'Select a Hero'}
            </button>
          </div>
        </div>
      </div>
    `;

    // Setup click handlers for cards
    const cards = this.overlay.querySelectorAll('.hb-hero-card:not(.locked)');
    cards.forEach((card) => {
      card.addEventListener('click', () => {
        const hid = card.getAttribute('data-hero-id');
        if (hid) {
          this.selectedHeroId = hid;
          this.session?.selectHero(hid);
          // Optimistically update
          this.update(lobby);
        }
      });
    });

    const lockBtn = document.getElementById('btn-lock-in-hero');
    lockBtn?.addEventListener('click', () => {
      if (this.selectedHeroId) {
        this.session?.selectHero(this.selectedHeroId);
      }
    });
  }
}
