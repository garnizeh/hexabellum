import * as PIXI from 'pixi.js';
import { HexRenderer, HEX_SIZE } from './renderer';
import { Animator } from './animator';
import {
  getPlayerState,
  getMoveTargets,
  getAttackTargets,
  findPath,
  setMoveOrder,
  setAttackOrder,
  setWaitOrder,
  endTurn,
  UnitData,
  HexCoord,
  GameState,
} from './bridge';
import { ClientSession } from './client_session';
import { SpellTargetDto } from './types';

export type InputMode = 'normal' | 'targetingSpell' | 'targetingRepair';

export class InputHandler {
  private selectedUnit: number | null = null;
  private renderer: HexRenderer;
  private session: ClientSession | null = null;
  private isResolving: boolean = false;
  private onTurnComplete: (() => void) | null = null;
  private stagedMoves = new Map<number, HexCoord>();
  private clickInterceptor: ((x: number, y: number) => boolean) | null = null;
  private inputMode: InputMode = 'normal';
  private onUnitSelectedChange: ((unit: UnitData | null) => void) | null = null;

  constructor(renderer: HexRenderer, session: ClientSession | null = null) {
    this.renderer = renderer;
    this.session = session;
    this.setupListeners();
    this.setupKeyBindings();
  }

  setClickInterceptor(cb: ((x: number, y: number) => boolean) | null): void {
    this.clickInterceptor = cb;
  }

  setSession(session: ClientSession | null): void {
    this.session = session;
    this.reset();
  }

  setOnTurnComplete(cb: () => void): void {
    this.onTurnComplete = cb;
  }

  setOnUnitSelectedChange(cb: (unit: UnitData | null) => void): void {
    this.onUnitSelectedChange = cb;
  }

  getSelectedUnitId(): number | null {
    return this.selectedUnit;
  }

  getSelectedUnit(): UnitData | null {
    if (this.selectedUnit === null) return null;
    const state = this.getState();
    return state.units[this.selectedUnit] ?? null;
  }

  reset(): void {
    this.selectedUnit = null;
    this.inputMode = 'normal';
    this.stagedMoves.clear();
    this.isResolving = false;
    this.renderer.clearOverlays();
    this.updateInspector(null);
    this.onUnitSelectedChange?.(null);
  }

  getIsResolving(): boolean {
    return this.isResolving;
  }

  private getState(): GameState {
    if (this.session) {
      return this.session.getGameState() ?? getPlayerState(0);
    }
    return getPlayerState(0);
  }

  private getPlayerTeam(): number {
    return this.session ? this.session.getCurrentTeam() : 0;
  }

  private getReachableTargets(unitId: number): { q: number; r: number; cost: number }[] {
    if (this.session) {
      return this.session.getMoveTargets(unitId);
    }
    return getMoveTargets(unitId);
  }

  private getAttackableTargets(unitId: number, fromQ: number, fromR: number): number[] {
    if (this.session) {
      return this.session.getAttackTargets(unitId, fromQ, fromR);
    }
    return getAttackTargets(unitId, fromQ, fromR);
  }

  private computePath(fromQ: number, fromR: number, toQ: number, toR: number): HexCoord[] {
    if (this.session) {
      return this.session.findPath(fromQ, fromR, toQ, toR);
    }
    return findPath(fromQ, fromR, toQ, toR);
  }

  private setupListeners(): void {
    const app = this.renderer.getApp();
    const stage = this.renderer.getStage();

    stage.eventMode = 'static';
    stage.hitArea = new PIXI.Rectangle(0, 0, app.screen.width, app.screen.height);

    stage.on('pointerdown', (e: PIXI.FederatedPointerEvent) => {
      if (this.isResolving) return;
      this.handleClick(e.global.x, e.global.y);
    });

    stage.on('pointermove', (e: PIXI.FederatedPointerEvent) => {
      if (this.isResolving || this.selectedUnit === null) return;
      this.handlePointerMove(e.global.x, e.global.y);
    });
  }

  private setupKeyBindings(): void {
    window.addEventListener('keydown', (e: KeyboardEvent) => {
      if (this.isResolving) return;
      const targetTag = (e.target as HTMLElement)?.tagName?.toLowerCase();
      if (targetTag === 'input' || targetTag === 'textarea') return;

      if (e.key === 'q' || e.key === 'Q') {
        this.triggerAbility();
      } else if (e.key === 'f' || e.key === 'F') {
        this.triggerRepair();
      } else if (e.key === 'a' || e.key === 'A') {
        this.triggerAttackMode();
      } else if (e.code === 'Space') {
        e.preventDefault();
        const endBtn = document.getElementById('end-turn-btn') as HTMLButtonElement;
        if (endBtn && !endBtn.disabled) {
          endBtn.click();
        }
      } else if (e.key === 'Escape') {
        this.cancelTargeting();
      }
    });
  }

  private handlePointerMove(x: number, y: number): void {
    const hex = this.pixelToHex(x, y);
    if (!hex || this.selectedUnit === null) {
      this.renderer.clearRaycastLine();
      return;
    }

    const state = this.getState();
    const selected = state.units[this.selectedUnit];
    if (!selected) return;

    const staged = this.stagedMoves.get(this.selectedUnit);
    const origin = staged ?? selected.pos;

    // Check if hovering an enemy or target in spell/attack mode
    const hoveredUnit = Object.values(state.units).find(u => u.pos.q === hex.q && u.pos.r === hex.r && u.hp > 0);

    if (hoveredUnit && hoveredUnit.id !== this.selectedUnit) {
      const blockers = this.session?.getVisionBlockers() ?? new Set(['0,2', '0,-2', '2,2', '-2,-2']);
      const isBlocked = !this.hasLOS(blockers, origin, hoveredUnit.pos);
      this.renderer.drawRaycastLine(origin, hoveredUnit.pos, isBlocked);
    } else {
      this.renderer.clearRaycastLine();
    }
  }

  private hasLOS(blockers: Set<string>, a: HexCoord, b: HexCoord): boolean {
    const dist = Math.max(
      Math.abs(a.q - b.q),
      Math.abs(a.r - b.r),
      Math.abs(-a.q - a.r - (-b.q - b.r))
    );
    if (dist <= 1) return true;

    for (let i = 1; i < dist; i++) {
      const t = i / dist;
      const q = a.q + (b.q - a.q) * t + 1e-6;
      const r = a.r + (b.r - a.r) * t + 1e-6;
      const s = -q - r;

      let rq = Math.round(q);
      let rr = Math.round(r);
      const rs = Math.round(s);
      const qDiff = Math.abs(rq - q);
      const rDiff = Math.abs(rr - r);
      const sDiff = Math.abs(rs - s);
      if (qDiff > rDiff && qDiff > sDiff) {
        rq = -rr - rs;
      } else if (rDiff > sDiff) {
        rr = -rq - rs;
      }
      if (blockers.has(`${rq},${rr}`)) {
        return false;
      }
    }
    return true;
  }

  private handleClick(x: number, y: number): void {
    if (this.clickInterceptor && this.clickInterceptor(x, y)) {
      return;
    }

    const hex = this.pixelToHex(x, y);
    if (!hex) return;

    const state = this.getState();
    if (state.phase === 'MatchEnd') return;

    const playerTeam = this.getPlayerTeam();

    // Mode: Targeting Spell
    if (this.inputMode === 'targetingSpell' && this.selectedUnit !== null) {
      const selected = state.units[this.selectedUnit];
      const spellId = this.getHeroSpellId(selected);
      if (spellId && this.session) {
        const staged = this.stagedMoves.get(this.selectedUnit);
        const origin = staged ?? selected.pos;
        const targets = this.session.getSpellTargets(this.selectedUnit, spellId, origin.q, origin.r);

        // Clicked on a target unit?
        const clickedUnit = Object.values(state.units).find(u => u.pos.q === hex.q && u.pos.r === hex.r && u.hp > 0);
        if (clickedUnit && targets.validUnitIds.includes(clickedUnit.id)) {
          const spellTarget: SpellTargetDto = {
            type: 'Unit',
            payload: { unit_id: clickedUnit.id },
          };
          const ok = this.session.stageCastOrder(this.selectedUnit, spellId, spellTarget);
          if (ok) {
            this.showToast(`Cast ${spellId.toUpperCase()} targeted on #${clickedUnit.id}`);
          }
          this.cancelTargeting();
          return;
        } else if (clickedUnit && targets.obstructedUnitIds.includes(clickedUnit.id)) {
          this.showToast('Line of sight obstructed by wall/smoke!');
          return;
        }
      }
      this.cancelTargeting();
      return;
    }

    // Mode: Targeting Repair
    if (this.inputMode === 'targetingRepair' && this.selectedUnit !== null) {
      if (this.session) {
        const staged = this.stagedMoves.get(this.selectedUnit);
        const origin = staged ?? state.units[this.selectedUnit].pos;
        const targets = this.session.getRepairTargets(this.selectedUnit, origin.q, origin.r);

        const clickedUnit = Object.values(state.units).find(u => u.pos.q === hex.q && u.pos.r === hex.r);
        if (clickedUnit && targets.includes(clickedUnit.id)) {
          const ok = this.session.stageRepairOrder(this.selectedUnit, clickedUnit.id);
          if (ok) {
            this.showToast(`Repair order queued on ${clickedUnit.kind} #${clickedUnit.id}`);
          }
          this.cancelTargeting();
          return;
        }
      }
      this.cancelTargeting();
      return;
    }

    // Normal Mode:
    // 1. Click on a living Player Hero: select
    for (const [idStr, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === playerTeam && unit.kind === 'Hero') {
        this.selectedUnit = Number(idStr);
        this.inputMode = 'normal';
        this.renderNormalOverlays(this.selectedUnit, state);
        this.renderer.highlightUnit(this.selectedUnit);
        this.updateInspector(unit);
        this.onUnitSelectedChange?.(unit);
        return;
      }
    }

    // 2. If a hero is selected: execute attack or move
    if (this.selectedUnit !== null) {
      const selected = state.units[this.selectedUnit];
      if (!selected) return;

      const staged = this.stagedMoves.get(this.selectedUnit);
      const origin = staged ?? selected.pos;
      const validAttacks = this.getAttackableTargets(this.selectedUnit, origin.q, origin.r);

      // Click on visible enemy in attack range -> attack
      for (const [idStr, unit] of Object.entries(state.units)) {
        if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team !== playerTeam) {
          const targetId = Number(idStr);
          if (validAttacks.includes(targetId)) {
            let ok = true;
            if (this.session) {
              ok = this.session.stageAttackOrder(this.selectedUnit, targetId);
            } else {
              setAttackOrder(this.selectedUnit, targetId);
            }
            if (!ok) {
              this.showToast('Action exceeds AP limit!');
              return;
            }
            this.selectedUnit = null;
            this.renderer.clearOverlays();
            this.updateInspector(null);
            this.onUnitSelectedChange?.(null);
            return;
          }
        }
      }

      // Click on reachable hex -> move
      const reachable = this.getReachableTargets(this.selectedUnit);
      const isReachable = reachable.some(t => t.q === hex.q && t.r === hex.r);
      const isSelf = hex.q === selected.pos.q && hex.r === selected.pos.r;

      if (isReachable || isSelf) {
        let success = true;
        if (this.session) {
          success = this.session.stageMoveOrder(this.selectedUnit, hex.q, hex.r);
        } else {
          success = setMoveOrder(this.selectedUnit, hex.q, hex.r);
        }

        if (success) {
          this.stagedMoves.set(this.selectedUnit, hex);
          const postMoveAttacks = this.getAttackableTargets(this.selectedUnit, hex.q, hex.r);
          this.renderer.clearOverlays();
          if (!isSelf) {
            const path = this.computePath(selected.pos.q, selected.pos.r, hex.q, hex.r);
            this.renderer.drawPath(path);
          }
          if (postMoveAttacks.length > 0) {
            this.renderer.drawAttackTargets(postMoveAttacks, state);
            this.renderer.highlightUnit(this.selectedUnit);
          } else {
            this.selectedUnit = null;
            this.updateInspector(null);
            this.onUnitSelectedChange?.(null);
          }
          return;
        }
      }

      this.selectedUnit = null;
      this.renderer.clearOverlays();
      this.updateInspector(null);
      this.onUnitSelectedChange?.(null);
    }

    // 3. Inspect any unit on the clicked tile
    for (const unit of Object.values(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r) {
        this.updateInspector(unit);
        return;
      }
    }
    this.updateInspector(null);
  }

  private renderNormalOverlays(unitId: number, state: GameState): void {
    const reachable = this.getReachableTargets(unitId);
    this.renderer.clearOverlays();
    this.renderer.drawMoveTargets(reachable);

    const staged = this.stagedMoves.get(unitId);
    const origin = staged ?? state.units[unitId].pos;
    const validAttacks = this.getAttackableTargets(unitId, origin.q, origin.r);
    this.renderer.drawAttackTargets(validAttacks, state);

    if (staged && (staged.q !== state.units[unitId].pos.q || staged.r !== state.units[unitId].pos.r)) {
      const path = this.computePath(state.units[unitId].pos.q, state.units[unitId].pos.r, staged.q, staged.r);
      this.renderer.drawPath(path);
    }
  }

  triggerAbility(): void {
    if (this.selectedUnit === null) {
      this.showToast('Select a hero first to use their ability!');
      return;
    }
    const state = this.getState();
    const hero = state.units[this.selectedUnit];
    if (!hero || hero.hp === 0) return;

    const spellId = this.getHeroSpellId(hero);
    if (!spellId) return;

    // Check cooldown
    const cd = hero.cooldowns?.[spellId] ?? 0;
    if (cd > 0) {
      this.showToast(`${spellId.toUpperCase()} is on cooldown (${cd} rounds remaining)`);
      return;
    }

    const energyCost = spellId === 'cleave' ? 3 : 2;
    if ((hero.energy ?? 0) < energyCost) {
      this.showToast(`Insufficient Energy! (Requires ⚡${energyCost}, you have ⚡${hero.energy ?? 0})`);
      return;
    }

    if (hero.ap < 1) {
      this.showToast('Insufficient AP to cast ability!');
      return;
    }

    if (spellId === 'cleave') {
      // Vanguard Cleave is Self-AOE: stage immediately!
      if (this.session) {
        const ok = this.session.stageCastOrder(this.selectedUnit, 'cleave', { type: 'None' });
        if (ok) {
          this.showToast('⚔️ CLEAVE queued! (Melee AOE wave)');
          this.renderer.clearOverlays();
          this.selectedUnit = null;
          this.onUnitSelectedChange?.(null);
        }
      }
      return;
    }

    // Ranger Bolt / Warden Mend: Switch to targeting mode
    this.inputMode = 'targetingSpell';
    const staged = this.stagedMoves.get(this.selectedUnit);
    const origin = staged ?? hero.pos;
    if (this.session) {
      const targets = this.session.getSpellTargets(this.selectedUnit, spellId, origin.q, origin.r);
      this.renderer.clearOverlays();
      this.renderer.drawAbilityTargets(targets.validUnitIds, targets.obstructedUnitIds, state);
      this.renderer.highlightUnit(this.selectedUnit);
      this.showToast(`Target ${spellId.toUpperCase()} (Cyan: In range, Red: Obstructed)`);
    }
  }

  triggerRepair(): void {
    if (this.selectedUnit === null) {
      this.showToast('Select a hero first to repair structures!');
      return;
    }
    const state = this.getState();
    const hero = state.units[this.selectedUnit];
    if (!hero || hero.hp === 0 || hero.ap < 1) {
      this.showToast('Hero needs at least 1 AP to repair.');
      return;
    }

    if (this.session) {
      const staged = this.stagedMoves.get(this.selectedUnit);
      const origin = staged ?? hero.pos;
      const targets = this.session.getRepairTargets(this.selectedUnit, origin.q, origin.r);
      if (targets.length === 0) {
        this.showToast('No adjacent damaged allied towers/spawners to repair.');
        return;
      }

      this.inputMode = 'targetingRepair';
      this.renderer.clearOverlays();
      this.renderer.drawRepairTargets(targets, state);
      this.renderer.highlightUnit(this.selectedUnit);
      this.showToast('Click an adjacent damaged structure to repair (+20 HP)');
    }
  }

  triggerAttackMode(): void {
    if (this.selectedUnit === null) {
      this.showToast('Select a hero first to attack!');
      return;
    }
    const state = this.getState();
    this.renderNormalOverlays(this.selectedUnit, state);
  }

  cancelTargeting(): void {
    this.inputMode = 'normal';
    this.renderer.clearRaycastLine();
    if (this.selectedUnit !== null) {
      const state = this.getState();
      this.renderNormalOverlays(this.selectedUnit, state);
      this.renderer.highlightUnit(this.selectedUnit);
    } else {
      this.renderer.clearOverlays();
    }
  }

  private getHeroSpellId(hero: UnitData): string | null {
    if (hero.cooldowns?.cleave !== undefined || hero.max_hp === 140) return 'cleave';
    if (hero.cooldowns?.bolt !== undefined || hero.attack_range >= 2) return 'bolt';
    if (hero.cooldowns?.mend !== undefined || hero.max_energy === 6) return 'mend';
    return 'cleave';
  }

  private showToast(msg: string): void {
    const toast = document.getElementById('hud-toast');
    if (toast) {
      toast.textContent = msg;
      toast.className = 'show';
      setTimeout(() => toast.classList.remove('show'), 2400);
    }
  }

  private updateInspector(unit: UnitData | null): void {
    const el = document.getElementById('unit-inspector');
    if (!el) return;
    if (!unit) {
      el.style.display = 'none';
      return;
    }
    el.style.display = 'block';
    const setVal = (id: string, text: string) => {
      const field = document.getElementById(id);
      if (field) field.textContent = text;
    };
    setVal('insp-name', `${unit.kind} #${unit.id}`);
    const teamLabel =
      unit.team === 255
        ? 'Neutral Guardian'
        : unit.team === this.getPlayerTeam()
        ? 'Blue (Player)'
        : 'Red (Enemy)';
    setVal('insp-team', teamLabel);
    setVal('insp-hp', `${unit.hp} / ${unit.max_hp}`);
    if ((unit.kind === 'Spawner' || unit.kind === 'SpawnerTower') && unit.spawn_interval) {
      setVal('insp-ap', `Wave: ${unit.spawn_counter}/${unit.spawn_interval}`);
    } else {
      setVal('insp-ap', `${unit.ap} / ${unit.max_ap}`);
    }
    setVal('insp-init', `⚡${unit.initiative}`);
    setVal('insp-dmg', `${unit.attack_damage}`);
    setVal('insp-range', `${unit.attack_range}`);
  }

  endTurnWithAnimation(animator: Animator, onFinished: () => void): void {
    if (this.isResolving) return;
    this.isResolving = true;
    this.selectedUnit = null;
    this.stagedMoves.clear();
    this.renderer.clearOverlays();
    this.updateInspector(null);
    this.onUnitSelectedChange?.(null);

    if (this.session) {
      this.session.submitOrders();
      return;
    }

    const events = endTurn();
    animator.playEvents(events, () => {
      this.isResolving = false;
      onFinished();
      this.onTurnComplete?.();
    });
  }

  handleServerResolved(events: any[], animator: Animator, onFinished: () => void): void {
    this.isResolving = true;
    this.selectedUnit = null;
    this.stagedMoves.clear();
    this.renderer.clearOverlays();
    this.updateInspector(null);
    this.onUnitSelectedChange?.(null);

    animator.playEvents(events, () => {
      this.isResolving = false;
      onFinished();
      this.onTurnComplete?.();
    });
  }

  private pixelToHex(x: number, y: number): { q: number; r: number } | null {
    const app = this.renderer.getApp();
    const cx = x - app.screen.width / 2;
    const cy = y - app.screen.height / 2;

    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / HEX_SIZE;
    const r = ((2 / 3) * cy) / HEX_SIZE;
    return this.hexRound(q, r);
  }

  private hexRound(q: number, r: number): { q: number; r: number } {
    const s = -q - r;
    let rq = Math.round(q);
    let rr = Math.round(r);
    const rs = Math.round(s);

    const qDiff = Math.abs(rq - q);
    const rDiff = Math.abs(rr - r);
    const sDiff = Math.abs(rs - s);

    if (qDiff > rDiff && qDiff > sDiff) {
      rq = -rr - rs;
    } else if (rDiff > sDiff) {
      rr = -rq - rs;
    }
    return { q: rq, r: rr };
  }
}
