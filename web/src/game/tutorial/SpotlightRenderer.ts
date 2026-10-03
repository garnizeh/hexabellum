import * as PIXI from 'pixi.js';
import { HEX_SIZE } from '../renderer';

export class SpotlightRenderer {
  private app: PIXI.Application;
  private container = new PIXI.Container();
  private ringGraphics = new PIXI.Graphics();
  private activeHexes: { q: number; r: number }[] = [];
  private ringColor: number = 0x00e676;
  private animTime: number = 0;
  private tickerFn: (() => void) | null = null;

  constructor(app: PIXI.Application) {
    this.app = app;
    this.container.zIndex = 50;
    this.container.addChild(this.ringGraphics);
    this.app.stage.addChild(this.container);

    this.tickerFn = () => {
      if (this.activeHexes.length > 0) {
        this.animTime += 0.05;
        this.renderRings();
      }
    };
    this.app.ticker.add(this.tickerFn);
  }

  public setSpotlights(hexes: { q: number; r: number }[], hexColorStr: string = '#00e676') {
    this.activeHexes = hexes;
    const cleanHex = hexColorStr.replace('#', '');
    this.ringColor = parseInt(cleanHex, 16) || 0x00e676;
    this.renderRings();
  }

  public clearSpotlights() {
    this.activeHexes = [];
    this.ringGraphics.clear();
  }

  public hexToPixel(q: number, r: number): { x: number; y: number } {
    const x = HEX_SIZE * (Math.sqrt(3) * q + (Math.sqrt(3) / 2) * r);
    const y = HEX_SIZE * (3 / 2) * r;
    return {
      x: x + this.app.screen.width / 2,
      y: y + this.app.screen.height / 2,
    };
  }

  private renderRings() {
    this.ringGraphics.clear();
    if (this.activeHexes.length === 0) return;

    const pulseScale = 1.0 + 0.12 * Math.sin(this.animTime * 3);
    const alpha = 0.65 + 0.35 * Math.sin(this.animTime * 3);

    for (const h of this.activeHexes) {
      const { x, y } = this.hexToPixel(h.q, h.r);
      const radius = (HEX_SIZE + 4) * pulseScale;

      // Outer glow
      this.ringGraphics.circle(x, y, radius + 4);
      this.ringGraphics.stroke({
        color: this.ringColor,
        alpha: alpha * 0.4,
        width: 4,
      });

      // Sharp inner ring
      this.ringGraphics.circle(x, y, radius);
      this.ringGraphics.stroke({
        color: this.ringColor,
        alpha: alpha,
        width: 2.5,
      });
    }
  }

  public destroy() {
    if (this.tickerFn) {
      this.app.ticker.remove(this.tickerFn);
      this.tickerFn = null;
    }
    this.ringGraphics.clear();
    this.container.removeFromParent();
  }
}
