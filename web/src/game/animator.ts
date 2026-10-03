import * as PIXI from 'pixi.js';
import { HexRenderer } from './renderer';
import { GameEvent, HexCoord } from './bridge';

export class Animator {
  private renderer: HexRenderer;
  private queue: GameEvent[] = [];
  private isPlaying: boolean = false;
  private onComplete: (() => void) | null = null;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
  }

  playEvents(events: GameEvent[], onComplete: () => void): void {
    this.queue = [...events];
    this.onComplete = onComplete;
    this.isPlaying = true;
    this.playNext();
  }

  private playNext(): void {
    if (this.queue.length === 0) {
      this.isPlaying = false;
      this.onComplete?.();
      return;
    }

    const event = this.queue.shift()!;
    switch (event.type) {
      case 'UnitMoved':
        this.animateMovement(event, () => this.playNext());
        break;
      case 'UnitAttacked':
      case 'TowerAttacked':
        this.animateAttack(event, () => this.playNext());
        break;
      case 'UnitDied':
        this.animateDeath(event, () => this.playNext());
        break;
      case 'UnitSpawned':
        this.animateSpawn(event, () => this.playNext());
        break;
      default:
        // Immediate events (RoundStarted, FogUpdated, etc.)
        this.playNext();
        break;
    }
  }

  private animateMovement(
    data: { unit_id: number; path: HexCoord[] },
    onDone: () => void
  ): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite || !data.path || data.path.length < 2) {
      onDone();
      return;
    }

    const points = data.path.map(hex => this.renderer.hexToPixel(hex.q, hex.r));
    const duration = 250 * (points.length - 1);
    const startTime = performance.now();

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      const totalSegments = points.length - 1;
      const segmentProgress = progress * totalSegments;
      const idx = Math.min(Math.floor(segmentProgress), totalSegments - 1);
      const t = segmentProgress - idx;

      const from = points[idx];
      const to = points[idx + 1];

      sprite.x = from.x + (to.x - from.x) * t;
      sprite.y = from.y + (to.y - from.y) * t;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        setTimeout(onDone, 40);
      }
    };

    requestAnimationFrame(animate);
  }

  private animateAttack(
    data: { target_id: number; damage: number; tower_id?: number; attacker_id?: number },
    onDone: () => void
  ): void {
    const targetSprite = this.renderer.getUnitSprite(data.target_id);
    const attackerId = data.tower_id ?? data.attacker_id;
    const attackerSprite = attackerId ? this.renderer.getUnitSprite(attackerId) : null;

    if (targetSprite) {
      // 1. Tower Laser Beam FX
      if (data.tower_id && attackerSprite) {
        const beam = new PIXI.Graphics();
        beam.moveTo(attackerSprite.x, attackerSprite.y);
        beam.lineTo(targetSprite.x, targetSprite.y);
        beam.stroke({ color: 0xff3d00, width: 4, alpha: 0.9 });
        this.renderer.getFxLayer().addChild(beam);

        setTimeout(() => {
          this.renderer.getFxLayer().removeChild(beam);
        }, 180);
      }

      // 2. Target Red Impact Flash
      const origAlpha = targetSprite.alpha;
      targetSprite.alpha = 0.4;
      this.showDamageText(targetSprite.x, targetSprite.y, data.damage);

      setTimeout(() => {
        targetSprite.alpha = origAlpha;
        setTimeout(onDone, 60);
      }, 200);
    } else {
      onDone();
    }
  }

  private animateDeath(data: { unit_id: number }, onDone: () => void): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite) {
      onDone();
      return;
    }

    const duration = 300;
    const startTime = performance.now();

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      sprite.alpha = 1 - progress;
      sprite.scale.set(1 - progress * 0.4);

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.removeUnitSprite(data.unit_id);
        setTimeout(onDone, 40);
      }
    };

    requestAnimationFrame(animate);
  }

  private animateSpawn(data: { unit_id: number }, onDone: () => void): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite) {
      onDone();
      return;
    }

    sprite.scale.set(0.1);
    const duration = 250;
    const startTime = performance.now();

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);
      sprite.scale.set(0.1 + progress * 0.9);

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        onDone();
      }
    };

    requestAnimationFrame(animate);
  }

  private showDamageText(x: number, y: number, damage: number): void {
    const text = new PIXI.Text({
      text: `-${damage}`,
      style: {
        fontSize: 18,
        fill: 0xff1744,
        fontWeight: 'bold',
        stroke: { color: 0x000000, width: 3 },
      },
    });
    text.anchor.set(0.5);
    text.x = x;
    text.y = y - 25;
    this.renderer.getFxLayer().addChild(text);

    const startTime = performance.now();
    const duration = 600;

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      text.y = y - 25 - progress * 30;
      text.alpha = 1 - progress;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.getFxLayer().removeChild(text);
      }
    };

    requestAnimationFrame(animate);
  }
}
