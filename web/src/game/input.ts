import * as PIXI from 'pixi.js';
import { HexRenderer, HEX_SIZE } from './renderer';
import { Animator } from './animator';
import {
  getPlayerState,
  getMoveTargets,
  getAttackTargets,
  setMoveOrder,
  setAttackOrder,
  setWaitOrder,
  endTurn,
  UnitData,
} from './bridge';

export class InputHandler {
  private selectedUnit: number | null = null;
  private renderer: HexRenderer;
  private isResolving: boolean = false;
  private onTurnComplete: (() => void) | null = null;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  setOnTurnComplete(cb: () => void): void {
    this.onTurnComplete = cb;
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
  }

  private handleClick(x: number, y: number): void {
    const hex = this.pixelToHex(x, y);
    if (!hex) return;

    const state = getPlayerState(0);
    if (state.phase === 'MatchEnd') return;

    // 1. Click on a living Player Hero: select
    for (const [idStr, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === 0 && unit.kind === 'Hero') {
        this.selectedUnit = Number(idStr);
        const reachable = getMoveTargets(this.selectedUnit);
        this.renderer.clearOverlays();
        this.renderer.drawMoveTargets(reachable);
        const validAttacks = getAttackTargets(this.selectedUnit, unit.pos.q, unit.pos.r);
        this.renderer.drawAttackTargets(validAttacks, state);
        this.renderer.highlightUnit(this.selectedUnit);
        this.updateInspector(unit);
        return;
      }
    }

    // 2. If a hero is selected: execute attack or move
    if (this.selectedUnit !== null) {
      const selected = state.units[this.selectedUnit];
      if (!selected) return;

      // Click on visible enemy in attack range -> attack
      const validAttacks = getAttackTargets(this.selectedUnit, selected.pos.q, selected.pos.r);
      for (const [idStr, unit] of Object.entries(state.units)) {
        if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team !== 0) {
          const targetId = Number(idStr);
          if (validAttacks.includes(targetId)) {
            setAttackOrder(this.selectedUnit, targetId);
            this.selectedUnit = null;
            this.renderer.clearOverlays();
            this.updateInspector(null);
            return;
          }
        }
      }

      // Click on reachable hex -> move
      const reachable = getMoveTargets(this.selectedUnit);
      const isReachable = reachable.some(t => t.q === hex.q && t.r === hex.r);
      const isSelf = hex.q === selected.pos.q && hex.r === selected.pos.r;

      if (isReachable || isSelf) {
        const success = setMoveOrder(this.selectedUnit, hex.q, hex.r);
        if (success) {
          const postMoveAttacks = getAttackTargets(this.selectedUnit, hex.q, hex.r);
          this.renderer.clearOverlays();
          if (!isSelf) {
            this.renderer.drawPath([selected.pos, hex]);
          }
          if (postMoveAttacks.length > 0) {
            this.renderer.drawAttackTargets(postMoveAttacks, state);
          } else {
            this.selectedUnit = null;
            this.updateInspector(null);
          }
        }
      } else {
        this.selectedUnit = null;
        this.renderer.clearOverlays();
        this.updateInspector(null);
      }
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
    setVal('insp-team', unit.team === 0 ? 'Blue (Player)' : 'Red (AI)');
    setVal('insp-hp', `${unit.hp} / ${unit.max_hp}`);
    setVal('insp-ap', `${unit.ap} / ${unit.max_ap}`);
    setVal('insp-init', `⚡${unit.initiative}`);
    setVal('insp-dmg', `${unit.attack_damage}`);
    setVal('insp-range', `${unit.attack_range}`);
  }

  endTurnWithAnimation(animator: Animator, onFinished: () => void): void {
    if (this.isResolving) return;
    this.isResolving = true;
    this.selectedUnit = null;
    this.renderer.clearOverlays();
    this.updateInspector(null);

    const events = endTurn();
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
