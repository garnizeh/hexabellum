import * as PIXI from 'pixi.js';
import { HexCoord, GameState } from './bridge';

// Hex geometry constants
const HEX_SIZE = 30; // radius in pixels

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer: PIXI.Container;
  private unitLayer: PIXI.Container;
  private overlayLayer: PIXI.Container;

  constructor(canvas: HTMLCanvasElement) {
    this.app = new PIXI.Application({
      view: canvas,
      width: window.innerWidth,
      height: window.innerHeight,
      background: '#1a1a2e',
      antialias: true,
    });

    this.hexLayer = new PIXI.Container();
    this.unitLayer = new PIXI.Container();
    this.overlayLayer = new PIXI.Container();

    this.app.stage.addChild(this.hexLayer);
    this.app.stage.addChild(this.overlayLayer);
    this.app.stage.addChild(this.unitLayer);
  }

  /** Convert axial hex coords to pixel position. */
  hexToPixel(q: number, r: number): { x: number; y: number } {
    const x = HEX_SIZE * (Math.sqrt(3) * q + (Math.sqrt(3) / 2) * r);
    const y = HEX_SIZE * (3 / 2) * r;
    return {
      x: x + this.app.screen.width / 2,
      y: y + this.app.screen.height / 2,
    };
  }

  /** Draw the hex grid. */
  drawMap(hexes: HexCoord[], obstacles: HexCoord[]): void {
    this.hexLayer.removeChildren();

    const obstacleSet = new Set(obstacles.map(h => `${h.q},${h.r}`));

    for (const hex of hexes) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);
      const isObstacle = obstacleSet.has(`${hex.q},${hex.r}`);

      const g = new PIXI.Graphics();

      // Draw hexagon (pointy-top orientation matches hexToPixel math)
      const points: number[] = [];
      for (let i = 0; i < 6; i++) {
        const angle = (Math.PI / 3) * i - Math.PI / 6;
        points.push(x + HEX_SIZE * Math.cos(angle), y + HEX_SIZE * Math.sin(angle));
      }
      g.poly(points);

      if (isObstacle) {
        g.fill({ color: 0x2d2d44 });
        g.stroke({ color: 0x444466, width: 1 });
      } else {
        g.fill({ color: 0x16213e });
        g.stroke({ color: 0x0f3460, width: 1 });
      }

      this.hexLayer.addChild(g);
    }
  }

  /** Draw units. */
  drawUnits(state: GameState): void {
    this.unitLayer.removeChildren();

    for (const [_id, unit] of Object.entries(state.units)) {
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

      const g = new PIXI.Graphics();

      // Team colors
      const color = unit.team === 0 ? 0x4fc3f7 : 0xef5350;

      // Draw unit as circle
      g.circle(x, y, HEX_SIZE * 0.5);
      g.fill({ color });
      g.stroke({ color: 0xffffff, width: 2 });

      // HP bar
      const hpWidth = HEX_SIZE * 0.8;
      const hpHeight = 4;
      const hpX = x - hpWidth / 2;
      const hpY = y - HEX_SIZE * 0.7;
      const hpRatio = unit.hp / unit.max_hp;

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x333333 });
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x4caf50 : 0xff9800 });

      this.unitLayer.addChild(g);
    }
  }

  /** Highlight valid move targets. */
  drawMoveTargets(targets: HexCoord[]): void {
    this.overlayLayer.removeChildren();

    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);

      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.3);
      g.fill({ color: 0x4caf50, alpha: 0.5 });

      this.overlayLayer.addChild(g);
    }
  }

  /** Clear move highlights. */
  clearMoveTargets(): void {
    this.overlayLayer.removeChildren();
  }

  getStage(): PIXI.Container {
    return this.app.stage;
  }

  getApp(): PIXI.Application {
    return this.app;
  }
}
