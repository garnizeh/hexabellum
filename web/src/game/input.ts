import * as PIXI from 'pixi.js';
import { HexRenderer, HEX_SIZE } from './renderer';
import {
  getState,
  getMoveTargets,
  getAttackTargets,
  setMoveOrder,
  setAttackOrder,
  setWaitOrder,
  allUnitsOrdered,
  endTurn,
  restart,
  getMapHexes,
  getObstacles,
  HexCoord,
  MoveTarget,
  GameState,
  UnitData,
} from './bridge';

type InputMode = 'idle' | 'choosingMove' | 'choosingAction';

export class InputHandler {
  private renderer: HexRenderer;
  private mode: InputMode = 'idle';
  private selectedUnitId: number | null = null;
  private moveTargets: MoveTarget[] = [];
  private plannedMove: HexCoord | null = null;
  private attackTargets: number[] = [];

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  private setupListeners(): void {
    const stage = this.renderer.getApp().stage;
    stage.eventMode = 'static';
    stage.hitArea = this.renderer.getApp().screen;

    stage.on('pointerdown', (event: PIXI.FederatedPointerEvent) => {
      this.handleClick(event.global.x, event.global.y);
    });
  }

  private handleClick(screenX: number, screenY: number): void {
    const state = getState();
    if (state.phase === 'MatchEnd') return;

    const hex = this.pixelToHex(screenX, screenY);
    if (!hex) return;

    switch (this.mode) {
      case 'idle':
        this.handleUnitSelection(hex, state);
        break;
      case 'choosingMove':
        this.handleMoveSelection(hex, state);
        break;
      case 'choosingAction':
        this.handleActionSelection(hex, state);
        break;
    }
  }

  private handleUnitSelection(hex: HexCoord, state: GameState): void {
    for (const [idStr, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r && unit.team === 0 && unit.hp > 0) {
        const id = Number(idStr);
        this.selectedUnitId = id;
        this.mode = 'choosingMove';
        this.plannedMove = null;

        this.moveTargets = getMoveTargets(id);
        this.renderer.clearOverlays();
        this.renderer.highlightUnit(id);
        this.renderer.drawMoveTargets(this.moveTargets);

        this.updateInspector(unit);
        this.updateStatus(`Hero #${id} selected. Click a green hex to move or click self to stay.`);
        return;
      }
    }

    // Clicked empty ground
    this.selectedUnitId = null;
    this.mode = 'idle';
    this.renderer.clearOverlays();
    this.renderer.highlightUnit(null);
    this.updateInspector(null);
    this.updateStatus('Select a blue hero to issue orders.');
  }

  private handleMoveSelection(hex: HexCoord, state: GameState): void {
    if (this.selectedUnitId === null) return;
    const unit = state.units[this.selectedUnitId];
    if (!unit) return;

    const target = this.moveTargets.find(t => t.q === hex.q && t.r === hex.r);
    const clickedSelf = hex.q === unit.pos.q && hex.r === unit.pos.r;

    if (target || clickedSelf) {
      const dest = clickedSelf ? unit.pos : hex;
      setMoveOrder(this.selectedUnitId, dest.q, dest.r);
      this.plannedMove = dest;
      this.mode = 'choosingAction';

      // Draw planned move path
      if (!clickedSelf) {
        this.renderer.drawPath([unit.pos, dest]);
      }

      // Show attack targets from new destination
      this.attackTargets = getAttackTargets(this.selectedUnitId, dest.q, dest.r);
      this.renderer.drawAttackTargets(this.attackTargets, state);

      if (this.attackTargets.length > 0) {
        this.updateStatus(`Click a red reticle to Attack, or click elsewhere to Wait.`);
      } else {
        this.updateStatus(`No enemies in range. Click anywhere to confirm Wait action.`);
      }
    } else {
      // Re-select another unit or cancel
      this.handleUnitSelection(hex, state);
    }
  }

  private handleActionSelection(hex: HexCoord, state: GameState): void {
    if (this.selectedUnitId === null) return;

    // Check if clicked an enemy in range
    const targetUnit = Object.values(state.units).find(
      u => u.pos.q === hex.q && u.pos.r === hex.r && u.team === 1 && u.hp > 0
    );

    if (targetUnit && this.attackTargets.includes(targetUnit.id)) {
      setAttackOrder(this.selectedUnitId, targetUnit.id);
    } else {
      setWaitOrder(this.selectedUnitId);
    }

    this.finishUnitOrders();
  }

  private finishUnitOrders(): void {
    this.selectedUnitId = null;
    this.mode = 'idle';
    this.plannedMove = null;
    this.moveTargets = [];
    this.attackTargets = [];

    this.renderer.clearOverlays();
    this.renderer.highlightUnit(null);
    this.updateInspector(null);

    if (allUnitsOrdered()) {
      this.updateStatus('All heroes have orders! Click "End Turn" to resolve.');
      const endBtn = document.getElementById('end-turn') as HTMLButtonElement;
      if (endBtn) endBtn.classList.add('ready');
    } else {
      this.updateStatus('Order confirmed. Select another hero.');
    }
  }

  public endTurn(): void {
    const events = endTurn();
    const state = getState();

    this.renderer.clearOverlays();
    this.renderer.drawUnits(state);

    const endBtn = document.getElementById('end-turn') as HTMLButtonElement;
    if (endBtn) endBtn.classList.remove('ready');

    if (state.phase === 'MatchEnd') {
      const winner = state.winner;
      const overlay = document.getElementById('game-over-modal') as HTMLElement;
      const title = document.getElementById('game-over-title') as HTMLElement;
      if (overlay && title) {
        overlay.style.display = 'flex';
        title.textContent = winner === 0 ? 'VICTORY' : 'DEFEAT';
        title.className = winner === 0 ? 'victory' : 'defeat';
      }
      this.updateStatus(`Match concluded! ${winner === 0 ? 'Player wins!' : 'AI wins!'}`);
    } else {
      this.updateStatus(`Round ${state.round} — Planning Phase. Select your heroes.`);
    }

    const roundEl = document.getElementById('round-counter');
    if (roundEl) roundEl.textContent = `Round ${state.round}`;

    console.log('Turn events:', events);
  }

  public restartGame(): void {
    restart();
    const state = getState();
    const hexes = getMapHexes();
    const obstacles = getObstacles();

    this.renderer.drawMap(hexes, obstacles);
    this.renderer.drawUnits(state);
    this.renderer.clearOverlays();

    const overlay = document.getElementById('game-over-modal') as HTMLElement;
    if (overlay) overlay.style.display = 'none';

    const roundEl = document.getElementById('round-counter');
    if (roundEl) roundEl.textContent = `Round 0`;

    const endBtn = document.getElementById('end-turn') as HTMLButtonElement;
    if (endBtn) endBtn.classList.remove('ready');

    this.updateInspector(null);
    this.updateStatus('New match started. Select a blue hero.');
  }

  private pixelToHex(screenX: number, screenY: number): HexCoord {
    const app = this.renderer.getApp();
    const cx = screenX - app.screen.width / 2;
    const cy = screenY - app.screen.height / 2;

    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / HEX_SIZE;
    const r = ((2 / 3) * cy) / HEX_SIZE;
    return this.hexRound(q, r);
  }

  private hexRound(q: number, r: number): HexCoord {
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

  private updateStatus(text: string): void {
    const el = document.getElementById('status-message');
    if (el) el.textContent = text;
  }

  private updateInspector(unit: UnitData | null): void {
    const panel = document.getElementById('unit-inspector');
    if (!panel) return;

    if (!unit) {
      panel.style.display = 'none';
      return;
    }

    panel.style.display = 'block';
    const nameEl = document.getElementById('insp-name');
    if (nameEl) nameEl.textContent = `Hero #${unit.id}`;

    const teamEl = document.getElementById('insp-team');
    if (teamEl) teamEl.textContent = unit.team === 0 ? 'Blue (Player)' : 'Red (Enemy)';

    const hpEl = document.getElementById('insp-hp');
    if (hpEl) hpEl.textContent = `${unit.hp} / ${unit.max_hp}`;

    const apEl = document.getElementById('insp-ap');
    if (apEl) apEl.textContent = `${unit.ap} / ${unit.max_ap}`;

    const initEl = document.getElementById('insp-init');
    if (initEl) initEl.textContent = `${unit.initiative}`;

    const dmgEl = document.getElementById('insp-dmg');
    if (dmgEl) dmgEl.textContent = `${unit.attack_damage}`;

    const rangeEl = document.getElementById('insp-range');
    if (rangeEl) rangeEl.textContent = `${unit.attack_range}`;
  }
}
