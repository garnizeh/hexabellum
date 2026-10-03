import * as PIXI from 'pixi.js';
import { HexRenderer } from './renderer';
import { getState, setMoveOrder, getMoveTargets, endTurn } from './bridge';

export class InputHandler {
  private selectedUnit: number | null = null;
  private renderer: HexRenderer;
  private onTurnEnd: (() => void) | null = null;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
    this.setupListeners();
  }

  setOnTurnEnd(cb: () => void): void {
    this.onTurnEnd = cb;
  }

  private setupListeners(): void {
    const app = this.renderer.getApp();
    const stage = this.renderer.getStage();

    stage.eventMode = 'static';
    // Make the whole stage area clickable
    stage.hitArea = new PIXI.Rectangle(0, 0, app.screen.width, app.screen.height);

    stage.on('pointerdown', (event: PIXI.FederatedPointerEvent) => {
      const pos = event.global;
      this.handleClick(pos.x, pos.y);
    });
  }

  private handleClick(x: number, y: number): void {
    const state = getState();

    // Find clicked hex
    const hex = this.pixelToHex(x, y);
    if (!hex) return;

    // Check if clicked on a unit
    for (const [id, unit] of Object.entries(state.units)) {
      if (unit.pos.q === hex.q && unit.pos.r === hex.r) {
        if (unit.team === 0) {
          // Player team: select it and show move targets
          this.selectedUnit = Number(id);
          const targets = getMoveTargets(Number(id));
          this.renderer.drawMoveTargets(targets);
          return;
        }
      }
    }

    // If a unit is selected, try to move
    if (this.selectedUnit !== null) {
      const success = setMoveOrder(this.selectedUnit, hex.q, hex.r);
      if (success) {
        this.renderer.clearMoveTargets();
      }
    }
  }

  /** Convert pixel position to hex coords. */
  private pixelToHex(x: number, y: number): { q: number; r: number } | null {
    const HEX_SIZE = 30;
    const app = this.renderer.getApp();

    // Offset to center
    const cx = x - app.screen.width / 2;
    const cy = y - app.screen.height / 2;

    // Pixel to fractional axial (pointy-top)
    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / HEX_SIZE;
    const r = ((2 / 3) * cy) / HEX_SIZE;

    // Round to nearest hex
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

  endTurn(): void {
    const events = endTurn();
    this.selectedUnit = null;
    this.renderer.clearMoveTargets();

    // Redraw with new state
    const state = getState();
    this.renderer.drawUnits(state);

    this.onTurnEnd?.();

    console.log("Round events:", events);
  }

  getSelectedUnit(): number | null {
    return this.selectedUnit;
  }
}
