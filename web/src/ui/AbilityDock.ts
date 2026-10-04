import { UnitData } from '../game/bridge';
import { ClientSession } from '../game/client_session';
import { InputHandler } from '../game/input';
import { ItemDto } from '../game/types';

export class AbilityDock {
  private dockEl: HTMLElement | null;
  private heroTitleEl: HTMLElement | null;
  private apDisplayEl: HTMLElement | null;
  private energyDisplayEl: HTMLElement | null;
  private abilityBtn: HTMLButtonElement | null;
  private abilityNameEl: HTMLElement | null;
  private abilityCostEl: HTMLElement | null;
  private abilityCdOverlay: HTMLElement | null;
  private abilityCdText: HTMLElement | null;
  private repairBtn: HTMLButtonElement | null;
  private attackBtn: HTMLButtonElement | null;
  private waitBtn: HTMLButtonElement | null;
  private centerBtn: HTMLButtonElement | null;

  // Phase 6 Economy & Progression UI
  private ecoBarEl: HTMLElement | null = null;
  private goldValEl: HTMLElement | null = null;
  private levelBadgeEl: HTMLElement | null = null;
  private xpFillEl: HTMLElement | null = null;
  private xpTextEl: HTMLElement | null = null;
  private inventoryRackEl: HTMLElement | null = null;
  private shopBtn: HTMLButtonElement | null = null;

  private input: InputHandler | null = null;
  private session: ClientSession | null = null;
  private onCenterHeroRequest: (() => void) | null = null;
  private onOpenShopRequest: (() => void) | null = null;

  constructor() {
    this.dockEl = document.getElementById('ability-dock');
    this.heroTitleEl = document.getElementById('dock-hero-title');
    this.apDisplayEl = document.getElementById('dock-ap-display');
    this.energyDisplayEl = document.getElementById('dock-energy-display');
    this.abilityBtn = document.getElementById('btn-ability-q') as HTMLButtonElement;
    this.abilityNameEl = document.getElementById('btn-ability-name');
    this.abilityCostEl = document.getElementById('btn-ability-cost');
    this.abilityCdOverlay = document.getElementById('btn-ability-cd-overlay');
    this.abilityCdText = document.getElementById('btn-ability-cd-text');
    this.repairBtn = document.getElementById('btn-repair-f') as HTMLButtonElement;
    this.attackBtn = document.getElementById('btn-attack-a') as HTMLButtonElement;
    this.waitBtn = document.getElementById('btn-wait-space') as HTMLButtonElement;
    this.centerBtn = document.getElementById('btn-center-hero') as HTMLButtonElement;

    this.createEconomyBar();
    this.setupListeners();
  }

  setInputHandler(input: InputHandler): void {
    this.input = input;
  }

  setSession(session: ClientSession | null): void {
    this.session = session;
  }

  setOnCenterHero(cb: () => void): void {
    this.onCenterHeroRequest = cb;
  }

  setOnOpenShop(cb: () => void): void {
    this.onOpenShopRequest = cb;
  }

  private createEconomyBar(): void {
    if (!this.dockEl || document.getElementById('hb-ability-dock-eco')) return;

    const ecoBar = document.createElement('div');
    ecoBar.id = 'hb-ability-dock-eco';
    ecoBar.className = 'hb-hero-economy-dock';
    ecoBar.style.marginBottom = '8px';
    ecoBar.style.justifyContent = 'space-between';

    ecoBar.innerHTML = `
      <div style="display:flex; align-items:center; gap:12px;">
        <div id="hb-dock-level" class="hb-level-badge">LVL 1</div>
        <div class="hb-xp-container">
          <div class="hb-xp-bar">
            <div id="hb-dock-xp-fill" class="hb-xp-fill" style="width: 0%;"></div>
          </div>
          <div class="hb-xp-text">
            <span>XP</span>
            <span id="hb-dock-xp-text">0 / 50</span>
          </div>
        </div>
        <div id="hb-dock-gold" class="hb-gold-counter">🪙 50 G</div>
      </div>

      <div style="display:flex; align-items:center; gap:10px;">
        <div id="hb-dock-inventory" class="hb-inventory-rack">
          <div class="hb-inventory-slot" data-slot="0"><span class="empty-icon">+</span></div>
          <div class="hb-inventory-slot" data-slot="1"><span class="empty-icon">+</span></div>
          <div class="hb-inventory-slot" data-slot="2"><span class="empty-icon">+</span></div>
        </div>
        <button id="btn-dock-shop" class="hb-open-shop-btn" title="Toggle Field Shop [B]">
          <span>🛡️ SHOP</span>
          <span style="font-size:10px; opacity:0.8; font-family:'JetBrains Mono',monospace;">[B]</span>
        </button>
      </div>
    `;

    // Insert as first child of dockEl
    this.dockEl.insertBefore(ecoBar, this.dockEl.firstChild);

    this.ecoBarEl = ecoBar;
    this.goldValEl = ecoBar.querySelector('#hb-dock-gold');
    this.levelBadgeEl = ecoBar.querySelector('#hb-dock-level');
    this.xpFillEl = ecoBar.querySelector('#hb-dock-xp-fill');
    this.xpTextEl = ecoBar.querySelector('#hb-dock-xp-text');
    this.inventoryRackEl = ecoBar.querySelector('#hb-dock-inventory');
    this.shopBtn = ecoBar.querySelector('#btn-dock-shop') as HTMLButtonElement;

    this.shopBtn?.addEventListener('click', () => {
      this.onOpenShopRequest?.();
    });
  }

  private setupListeners(): void {
    this.abilityBtn?.addEventListener('click', () => {
      this.input?.triggerAbility();
    });

    this.repairBtn?.addEventListener('click', () => {
      this.input?.triggerRepair();
    });

    this.attackBtn?.addEventListener('click', () => {
      this.input?.triggerAttackMode();
    });

    this.waitBtn?.addEventListener('click', () => {
      this.input?.triggerWaitOrder();
    });

    this.centerBtn?.addEventListener('click', () => {
      this.onCenterHeroRequest?.();
    });
  }

  update(unit: UnitData | null): void {
    if (!this.dockEl) return;

    if (!unit || unit.kind !== 'Hero') {
      const isDead = this.session?.isHeroDead();
      if (isDead) {
        if (this.heroTitleEl) {
          const rounds = this.session?.getHeroRespawnRounds() ?? 3;
          this.heroTitleEl.textContent = `💀 HERO FALLEN — RESPAWNING IN ${rounds} ROUNDS`;
        }
        if (this.ecoBarEl) this.ecoBarEl.style.display = 'flex';
        this.updateShopButton(null);
      } else {
        if (this.heroTitleEl) this.heroTitleEl.textContent = 'NO HERO SELECTED';
        if (this.ecoBarEl) this.ecoBarEl.style.display = 'none';
      }
      if (this.apDisplayEl) this.apDisplayEl.textContent = 'AP: -';
      if (this.energyDisplayEl) this.energyDisplayEl.textContent = '⚡ -';
      if (this.abilityBtn) this.abilityBtn.disabled = true;
      if (this.repairBtn) this.repairBtn.disabled = true;
      if (this.attackBtn) this.attackBtn.disabled = true;
      if (this.waitBtn) this.waitBtn.disabled = true;
      if (this.abilityCdOverlay) this.abilityCdOverlay.style.display = 'none';
      return;
    }

    if (this.ecoBarEl) this.ecoBarEl.style.display = 'flex';

    const isDead = this.session?.isHeroDead() || unit.life_state === 'dead_awaiting_respawn';
    const heroSpell = this.getHeroSpell(unit);
    if (this.heroTitleEl) {
      if (isDead) {
        const rounds = unit.respawn_rounds ?? this.session?.getHeroRespawnRounds() ?? 3;
        this.heroTitleEl.textContent = `💀 ${heroSpell.heroClass} DEFEATED — RESPAWNING IN ${rounds} ROUNDS`;
      } else {
        this.heroTitleEl.textContent = `${heroSpell.heroClass} #${unit.id} — ${heroSpell.role}`;
      }
    }

    if (this.apDisplayEl) {
      this.apDisplayEl.textContent = `AP: ${unit.ap}/${unit.max_ap}`;
    }

    const energy = unit.energy ?? 0;
    const maxEnergy = unit.max_energy ?? 5;
    if (this.energyDisplayEl) {
      this.energyDisplayEl.textContent = `⚡ ${energy}/${maxEnergy}`;
    }

    // Ability (Q)
    if (this.abilityBtn && this.abilityNameEl && this.abilityCostEl) {
      this.abilityNameEl.textContent = heroSpell.spellName;
      this.abilityCostEl.textContent = `⚡${heroSpell.energyCost} · 1 AP`;

      const cd = unit.cooldowns?.[heroSpell.spellId] ?? 0;
      if (cd > 0) {
        if (this.abilityCdOverlay) {
          this.abilityCdOverlay.style.display = 'flex';
          if (this.abilityCdText) this.abilityCdText.textContent = `${cd}`;
        }
        this.abilityBtn.disabled = true;
      } else {
        if (this.abilityCdOverlay) this.abilityCdOverlay.style.display = 'none';
        const canAfford = energy >= heroSpell.energyCost && unit.ap >= 1;
        this.abilityBtn.disabled = !canAfford || isDead;
      }
      if (isDead) {
        this.abilityBtn.title = 'Hero is awaiting respawn at base';
      }
    }

    // Repair (F)
    if (this.repairBtn) {
      let hasRepairTarget = false;
      if (this.session && unit.ap >= 1 && !isDead) {
        const targets = this.session.getRepairTargets(unit.id, unit.pos.q, unit.pos.r);
        hasRepairTarget = targets.length > 0;
      }
      this.repairBtn.disabled = !hasRepairTarget || unit.ap < 1 || isDead;
      if (isDead) {
        this.repairBtn.title = 'Hero is awaiting respawn at base';
      }
    }

    // Attack (A)
    if (this.attackBtn) {
      this.attackBtn.disabled = unit.ap < 1 || isDead;
      if (isDead) {
        this.attackBtn.title = 'Hero is awaiting respawn at base';
      }
    }

    // Wait (Space)
    if (this.waitBtn) {
      this.waitBtn.disabled = isDead;
      if (isDead) {
        this.waitBtn.title = 'Hero is awaiting respawn at base';
      }
    }

    // Update Phase 6 Economy & Progression
    this.updateEconomyElements(unit);
    this.updateShopButton(unit);
  }

  private updateShopButton(unit: UnitData | null): void {
    if (!this.shopBtn) return;

    const isDead = this.session?.isHeroDead() || unit?.life_state === 'dead_awaiting_respawn';
    const phase = this.session?.getPhase() ?? 'Planning';
    const canShop = this.session?.canShop() ?? false;

    this.shopBtn.className = 'hb-open-shop-btn';

    if (phase === 'Resolution') {
      this.shopBtn.classList.add('resolving');
      this.shopBtn.innerHTML = `<span>SHOP LOCKED: RESOLUTION</span>`;
      this.shopBtn.title = 'Shop Unavailable: Orders currently resolving';
    } else if (isDead) {
      this.shopBtn.classList.add('hero-dead');
      this.shopBtn.innerHTML = `<span>SHOP LOCKED: HERO DEFEATED</span>`;
      this.shopBtn.title = 'Shop Unavailable: Awaiting respawn at base';
    } else if (canShop) {
      this.shopBtn.classList.add('in-base');
      this.shopBtn.innerHTML = `<span>BASE SHOP [B]</span>`;
      this.shopBtn.title = 'Base Shop Open: Buy passive items';
    } else {
      this.shopBtn.classList.add('outside-base');
      this.shopBtn.innerHTML = `<span>SHOP LOCKED: RETURN TO BASE</span>`;
      this.shopBtn.title = 'Shop Unavailable: You must be inside your team base to purchase items';
    }
  }

  private updateEconomyElements(unit: UnitData): void {
    const controlledEco = this.session?.getControlledHeroEconomy();
    const gold = controlledEco?.gold ?? (unit as any).gold ?? 50;
    const xp = controlledEco?.xp ?? (unit as any).xp ?? 0;
    const level = controlledEco?.level ?? (unit as any).level ?? 1;
    const items: string[] = controlledEco?.items ?? (unit as any).items ?? [];

    if (this.goldValEl) {
      this.goldValEl.textContent = `🪙 ${gold} G`;
    }

    if (this.levelBadgeEl) {
      this.levelBadgeEl.textContent = `LVL ${level}`;
      if (level >= 5) {
        this.levelBadgeEl.style.borderColor = '#ffd700';
        this.levelBadgeEl.style.color = '#ffd700';
      }
    }

    // Level XP Thresholds: L1=0, L2=50, L3=120, L4=220, L5=350
    const thresholds = [0, 50, 120, 220, 350];
    const prevThreshold = thresholds[Math.min(level - 1, 4)];
    const nextThreshold = level >= 5 ? 350 : thresholds[level];

    if (this.xpFillEl && this.xpTextEl) {
      if (level >= 5) {
        this.xpFillEl.style.width = '100%';
        this.xpTextEl.textContent = `${xp} (MAX)`;
      } else {
        const progressXp = Math.max(0, xp - prevThreshold);
        const neededXp = Math.max(1, nextThreshold - prevThreshold);
        const percent = Math.min(100, Math.round((progressXp / neededXp) * 100));
        this.xpFillEl.style.width = `${percent}%`;
        this.xpTextEl.textContent = `${xp} / ${nextThreshold}`;
      }
    }

    // Update 3 inventory slots
    if (this.inventoryRackEl) {
      let slotsHtml = '';
      for (let i = 0; i < 3; i++) {
        if (i < items.length) {
          const itemId = items[i];
          const info = this.getItemDisplayInfo(itemId);
          slotsHtml += `
            <div class="hb-inventory-slot occupied" title="${info.name}">
              <span>${info.icon}</span>
              <div class="tooltip">
                <strong>${info.name}</strong><br>
                <span style="color:#38bdf8;">${info.stat}</span>
              </div>
            </div>
          `;
        } else {
          slotsHtml += `
            <div class="hb-inventory-slot" title="Empty Slot">
              <span style="color:#64748b; font-size:14px;">+</span>
            </div>
          `;
        }
      }
      this.inventoryRackEl.innerHTML = slotsHtml;

      // Clicking any slot opens the shop drawer
      const slots = this.inventoryRackEl.querySelectorAll('.hb-inventory-slot');
      slots.forEach((s) => {
        s.addEventListener('click', () => {
          this.onOpenShopRequest?.();
        });
      });
    }
  }

  private getItemDisplayInfo(itemId: string): { name: string; icon: string; stat: string } {
    switch (itemId) {
      case 'longblade':
        return { name: 'Longblade', icon: '⚔️', stat: '+6 Attack Damage' };
      case 'plate_armor':
        return { name: 'Plate Armor', icon: '🛡️', stat: '+35 Max HP & Instant Heal' };
      case 'scout_lens':
        return { name: 'Scout Lens', icon: '👁️', stat: '+1 Vision Range' };
      case 'focus_charm':
        return { name: 'Focus Charm', icon: '🔮', stat: '+1 Energy Regen' };
      default:
        return { name: itemId, icon: '📦', stat: 'Equipped Item' };
    }
  }

  private getHeroSpell(unit: UnitData): {
    heroClass: string;
    role: string;
    spellId: string;
    spellName: string;
    energyCost: number;
  } {
    const hid = (unit.hero_id ?? '').toLowerCase();
    if (hid === 'vanguard' || unit.cooldowns?.cleave !== undefined || unit.max_hp === 140) {
      return {
        heroClass: 'VANGUARD',
        role: 'Frontline Cleaver',
        spellId: 'cleave',
        spellName: 'CLEAVE',
        energyCost: 3,
      };
    }
    if (hid === 'sniper' || unit.cooldowns?.longshot !== undefined || unit.attack_range >= 3 || unit.max_hp === 80) {
      return {
        heroClass: 'SNIPER',
        role: 'Artillery Marksman',
        spellId: 'longshot',
        spellName: 'LONGSHOT',
        energyCost: 3,
      };
    }
    if (hid === 'berserker' || unit.cooldowns?.fury !== undefined || (unit.max_hp === 120 && unit.attack_range === 1)) {
      return {
        heroClass: 'BERSERKER',
        role: 'Melee Rage Bruiser',
        spellId: 'fury',
        spellName: 'FURY',
        energyCost: 2,
      };
    }
    if (hid === 'ranger' || unit.cooldowns?.bolt !== undefined || (unit.attack_range >= 2 && unit.attack_range < 3)) {
      return {
        heroClass: 'RANGER',
        role: 'Ranged Sniper',
        spellId: 'bolt',
        spellName: 'BOLT',
        energyCost: 2,
      };
    }
    if (hid === 'warden' || unit.cooldowns?.mend !== undefined || unit.max_energy === 6) {
      return {
        heroClass: 'WARDEN',
        role: 'Combat Medic',
        spellId: 'mend',
        spellName: 'MEND',
        energyCost: 2,
      };
    }
    return {
      heroClass: 'HERO',
      role: 'Tactical Fighter',
      spellId: 'cleave',
      spellName: 'ABILITY',
      energyCost: 2,
    };
  }
}
