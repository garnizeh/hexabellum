import { UnitData } from '../game/bridge';
import { ClientSession } from '../game/client_session';
import { InputHandler } from '../game/input';

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

  private input: InputHandler | null = null;
  private session: ClientSession | null = null;

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

    this.setupListeners();
  }

  setInputHandler(input: InputHandler): void {
    this.input = input;
  }

  setSession(session: ClientSession | null): void {
    this.session = session;
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
  }

  update(unit: UnitData | null): void {
    if (!this.dockEl) return;

    if (!unit || unit.kind !== 'Hero') {
      if (this.heroTitleEl) this.heroTitleEl.textContent = 'NO HERO SELECTED';
      if (this.apDisplayEl) this.apDisplayEl.textContent = 'AP: -';
      if (this.energyDisplayEl) this.energyDisplayEl.textContent = '⚡ -';
      if (this.abilityBtn) this.abilityBtn.disabled = true;
      if (this.repairBtn) this.repairBtn.disabled = true;
      if (this.attackBtn) this.attackBtn.disabled = true;
      if (this.abilityCdOverlay) this.abilityCdOverlay.style.display = 'none';
      return;
    }

    const heroSpell = this.getHeroSpell(unit);
    if (this.heroTitleEl) {
      this.heroTitleEl.textContent = `${heroSpell.heroClass} #${unit.id} — ${heroSpell.role}`;
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
        this.abilityBtn.disabled = !canAfford;
      }
    }

    // Repair (F)
    if (this.repairBtn) {
      let hasRepairTarget = false;
      if (this.session && unit.ap >= 1) {
        const targets = this.session.getRepairTargets(unit.id, unit.pos.q, unit.pos.r);
        hasRepairTarget = targets.length > 0;
      }
      this.repairBtn.disabled = !hasRepairTarget || unit.ap < 1;
    }

    // Attack (A)
    if (this.attackBtn) {
      this.attackBtn.disabled = unit.ap < 1;
    }
  }

  private getHeroSpell(unit: UnitData): {
    heroClass: string;
    role: string;
    spellId: string;
    spellName: string;
    energyCost: number;
  } {
    if (unit.max_hp === 140 || unit.cooldowns?.cleave !== undefined) {
      return {
        heroClass: 'VANGUARD',
        role: 'Frontline Cleaver',
        spellId: 'cleave',
        spellName: 'CLEAVE',
        energyCost: 3,
      };
    }
    if (unit.attack_range >= 2 || unit.cooldowns?.bolt !== undefined) {
      return {
        heroClass: 'RANGER',
        role: 'Ranged Sniper',
        spellId: 'bolt',
        spellName: 'BOLT',
        energyCost: 2,
      };
    }
    if (unit.max_energy === 6 || unit.cooldowns?.mend !== undefined) {
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
