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

export class InputHandler {
  private selectedUnit: number | null = null;
  private renderer: HexRenderer;
  private session: ClientSession | null = null;
  private isResolving: boolean = false;
  private onTurnComplete: (() => void) | null = null;
  private stagedMoves = new Map<number, HexCoord>();

  constructor(renderer: HexRenderer, session: ClientSession | null = null) {
    this.renderer = renderer;
    this.session = session;
    this.setupListeners();
  }

  setSession(session: ClientSession | null): void {
    this.session = session;
    this.reset();
  }

  setOnTurnComplete(cb: () => void): void {
    this.onTurnComplete = cb;
  }

  reset(): void {
    this.selectedUnit = null;
    this.stagedMoves.clear();
    this.isResolving = false;
    this.renderer.clearOverlays();
    this.updateInspector(null);
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
  }

  private handleClick(x: number, y: number): void {
    const hex = this.pixelToHex(x, y);
    if (!hex) return;

    const state = this.getState();
    if (state.phase === 'MatchEnd') return;

    const playerTeam = this.getPlayerTeam();

    // 1. Click on a living Player Hero: select
    for (const [idStr, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === playerTeam && unit.kind === 'Hero') {
        this.selectedUnit = Number(idStr);
        const reachable = this.getReachableTargets(this.selectedUnit);
        this.renderer.clearOverlays();
        this.renderer.drawMoveTargets(reachable);

        const staged = this.stagedMoves.get(this.selectedUnit);
        const origin = staged ?? unit.pos;
        const validAttacks = this.getAttackableTargets(this.selectedUnit, origin.q, origin.r);
        this.renderer.drawAttackTargets(validAttacks, state);

        if (staged && (staged.q !== unit.pos.q || staged.r !== unit.pos.r)) {
          const path = this.computePath(unit.pos.q, unit.pos.r, staged.q, staged.r);
          this.renderer.drawPath(path);
        }

        this.renderer.highlightUnit(this.selectedUnit);
        this.updateInspector(unit);
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
              const toast = document.getElementById('hud-toast');
              if (toast) {
                toast.textContent = 'Action exceeds AP limit!';
                toast.className = 'show';
                setTimeout(() => toast.classList.remove('show'), 2000);
              }
              return;
            }
            this.selectedUnit = null;
            this.renderer.clearOverlays();
            this.updateInspector(null);
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
          }
          return;
        }
      }

      this.selectedUnit = null;
      this.renderer.clearOverlays();
      this.updateInspector(null);
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
    const teamLabel = unit.team === this.getPlayerTeam() ? 'Blue (Player)' : 'Red (Enemy)';
    setVal('insp-team', teamLabel);
    setVal('insp-hp', `${unit.hp} / ${unit.max_hp}`);
    if (unit.kind === 'SpawnerTower' && unit.spawn_interval) {
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

    if (this.session) {
      this.session.submitOrders();
      // Server will respond with RoundResolved, which will call animator
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
