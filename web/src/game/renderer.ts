import * as PIXI from 'pixi.js';
import { HexCoord, GameState, MoveTarget } from './bridge';

export const HEX_SIZE = 32;

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer = new PIXI.Container();
  private overlayLayer = new PIXI.Container();
  private pathLayer = new PIXI.Container();
  private unitLayer = new PIXI.Container();

  private unitSprites = new Map<number, PIXI.Container>();

  constructor(app: PIXI.Application) {
    this.app = app;
    this.app.stage.addChild(this.hexLayer);
    this.app.stage.addChild(this.overlayLayer);
    this.app.stage.addChild(this.pathLayer);
    this.app.stage.addChild(this.unitLayer);
  }

  hexToPixel(q: number, r: number): { x: number; y: number } {
    const x = HEX_SIZE * (Math.sqrt(3) * q + (Math.sqrt(3) / 2) * r);
    const y = HEX_SIZE * (3 / 2) * r;
    return {
      x: x + this.app.screen.width / 2,
      y: y + this.app.screen.height / 2,
    };
  }

  drawMap(hexes: HexCoord[], obstacles: HexCoord[]): void {
    this.hexLayer.removeChildren();
    const obstacleSet = new Set(obstacles.map(h => `${h.q},${h.r}`));

    for (const hex of hexes) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);
      const isObstacle = obstacleSet.has(`${hex.q},${hex.r}`);

      const g = new PIXI.Graphics();
      const points: number[] = [];
      for (let i = 0; i < 6; i++) {
        const angle = (Math.PI / 3) * i - Math.PI / 6;
        points.push(x + HEX_SIZE * Math.cos(angle), y + HEX_SIZE * Math.sin(angle));
      }
      g.poly(points);

      if (isObstacle) {
        g.fill({ color: 0x24243a });
        g.stroke({ color: 0x3d3d5c, width: 1.5 });
      } else {
        g.fill({ color: 0x121b2d });
        g.stroke({ color: 0x1f3453, width: 1 });
      }

      this.hexLayer.addChild(g);
    }
  }

  drawUnits(state: GameState): void {
    this.unitLayer.removeChildren();
    this.unitSprites.clear();

    for (const [idStr, unit] of Object.entries(state.units)) {
      const id = Number(idStr);
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

      const container = new PIXI.Container();
      container.x = x;
      container.y = y;

      const g = new PIXI.Graphics();
      const isPlayer = unit.team === 0;
      const baseColor = isPlayer ? 0x00d2ff : 0xff3366;

      // Outer glow & unit circle
      g.circle(0, 0, HEX_SIZE * 0.52);
      g.fill({ color: baseColor });
      g.stroke({ color: 0xffffff, width: 2 });

      // HP Bar (Above unit)
      const hpWidth = HEX_SIZE * 0.9;
      const hpHeight = 5;
      const hpX = -hpWidth / 2;
      const hpY = -HEX_SIZE * 0.75;
      const hpRatio = Math.max(0, Math.min(1, unit.hp / unit.max_hp));

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x111118 });
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x00e676 : hpRatio > 0.25 ? 0xffea00 : 0xff1744 });

      // AP Pip Dots (Below unit)
      const apY = HEX_SIZE * 0.72;
      for (let i = 0; i < unit.max_ap; i++) {
        const apX = (i - (unit.max_ap - 1) / 2) * 10;
        g.circle(apX, apY, 3);
        g.fill({ color: i < unit.ap ? 0xffd600 : 0x424242 });
      }

      // Initiative badge
      const initText = new PIXI.Text({
        text: `⚡${unit.initiative}`,
        style: {
          fontSize: 10,
          fill: 0xffffff,
          fontFamily: 'Outfit, Inter, sans-serif',
          fontWeight: 'bold',
        },
      });
      initText.anchor.set(0.5);
      initText.y = 0;

      container.addChild(g);
      container.addChild(initText);
      this.unitLayer.addChild(container);
      this.unitSprites.set(id, container);
    }
  }

  drawMoveTargets(targets: MoveTarget[]): void {
    this.overlayLayer.removeChildren();

    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);
      const g = new PIXI.Graphics();

      g.circle(x, y, HEX_SIZE * 0.45);
      g.fill({ color: 0x00e676, alpha: 0.35 });
      g.stroke({ color: 0x00e676, width: 2, alpha: 0.8 });

      const costText = new PIXI.Text({
        text: `${target.cost} AP`,
        style: {
          fontSize: 11,
          fill: 0xffffff,
          fontFamily: 'Outfit, Inter, sans-serif',
          fontWeight: 'bold',
        },
      });
      costText.anchor.set(0.5);
      costText.x = x;
      costText.y = y;

      this.overlayLayer.addChild(g);
      this.overlayLayer.addChild(costText);
    }
  }

  drawAttackTargets(targetIds: number[], state: GameState): void {
    for (const targetId of targetIds) {
      const unit = state.units[targetId];
      if (!unit) continue;

      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);
      const g = new PIXI.Graphics();

      // Pulsing crosshair ring
      g.circle(x, y, HEX_SIZE * 0.65);
      g.stroke({ color: 0xff1744, width: 3, alpha: 0.9 });

      this.overlayLayer.addChild(g);
    }
  }

  drawPath(path: HexCoord[]): void {
    this.pathLayer.removeChildren();
    if (path.length < 2) return;

    const g = new PIXI.Graphics();
    const start = this.hexToPixel(path[0].q, path[0].r);
    g.moveTo(start.x, start.y);

    for (let i = 1; i < path.length; i++) {
      const pt = this.hexToPixel(path[i].q, path[i].r);
      g.lineTo(pt.x, pt.y);
    }
    g.stroke({ color: 0xffd600, width: 3, alpha: 0.8 });
    this.pathLayer.addChild(g);
  }

  highlightUnit(unitId: number | null): void {
    for (const [id, container] of this.unitSprites) {
      const g = container.getChildAt(0) as PIXI.Graphics;
      if (id === unitId) {
        g.stroke({ color: 0xffd600, width: 3.5 });
      } else {
        g.stroke({ color: 0xffffff, width: 2 });
      }
    }
  }

  clearOverlays(): void {
    this.overlayLayer.removeChildren();
    this.pathLayer.removeChildren();
  }

  getApp(): PIXI.Application {
    return this.app;
  }
}
