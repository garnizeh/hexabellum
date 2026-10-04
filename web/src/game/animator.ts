import * as PIXI from 'pixi.js';
import { HexRenderer, HEX_SIZE } from './renderer';
import { GameEvent, HexCoord } from './bridge';
import { SanitizedGameEvent } from './types';

export class Animator {
  private renderer: HexRenderer;
  private queue: (GameEvent | SanitizedGameEvent)[] = [];
  private isPlaying: boolean = false;
  private onComplete: (() => void) | null = null;
  private playbackSpeed: number = 1.0;

  constructor(renderer: HexRenderer) {
    this.renderer = renderer;
  }

  getPlaybackSpeed(): number {
    return this.playbackSpeed;
  }

  setPlaybackSpeed(speed: number): void {
    this.playbackSpeed = Math.max(0.5, Math.min(3.0, speed));
  }

  getIsPlaying(): boolean {
    return this.isPlaying;
  }

  skipToEnd(): void {
    if (!this.isPlaying) return;

    // Instantly apply all remaining events in the resolution queue (docs/ui-ux.md §4.14 & §8.2)
    while (this.queue.length > 0) {
      const event = this.queue.shift()!;
      this.applyEventInstantly(event);
    }

    this.isPlaying = false;
    this.onComplete?.();
  }

  private applyEventInstantly(event: any): void {
    switch (event.type) {
      case 'UnitMoved': {
        const sprite = this.renderer.getUnitSprite(event.unit_id);
        if (sprite && event.path && event.path.length > 0) {
          const last = event.path[event.path.length - 1];
          const pos = this.renderer.hexToPixel(last.q, last.r);
          sprite.x = pos.x;
          sprite.y = pos.y;
        }
        break;
      }
      case 'UnitAttacked':
      case 'TowerAttacked': {
        if (event.target_hp_remaining !== undefined) {
          this.renderer.updateUnitHp(event.target_id, event.target_hp_remaining);
        }
        break;
      }
      case 'HealApplied':
      case 'StructureRepaired': {
        if (event.target_hp_remaining !== undefined) {
          this.renderer.updateUnitHp(event.target_id, event.target_hp_remaining);
        }
        break;
      }
      case 'BaseRegenerationApplied': {
        if (event.new_hp !== undefined) {
          this.renderer.updateUnitHp(event.unit_id, event.new_hp);
        }
        break;
      }
      case 'UnitDied': {
        this.renderer.removeUnitSprite(event.unit_id);
        break;
      }
      case 'CoreDestroyed': {
        this.renderer.removeUnitSprite(event.core_id);
        break;
      }
      case 'HeroRespawned': {
        const sprite = this.renderer.getUnitSprite(event.unit_id);
        if (sprite && event.pos) {
          const p = this.renderer.hexToPixel(event.pos.q, event.pos.r);
          sprite.x = p.x;
          sprite.y = p.y;
        }
        break;
      }
      default:
        break;
    }
  }

  playEvents(events: (GameEvent | SanitizedGameEvent)[], onComplete?: () => void): void {
    this.queue = [...events];
    this.onComplete = onComplete ?? null;
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
      case 'SpellCast':
        this.animateSpellCast(event, () => this.playNext());
        break;
      case 'HealApplied':
        this.animateHeal(event, () => this.playNext());
        break;
      case 'StructureRepaired':
        this.animateRepair(event, () => this.playNext());
        break;
      case 'TeamBuffApplied':
        this.animateTeamBuff(event, () => this.playNext());
        break;
      case 'NeutralCampCleared':
        this.animateCampCleared(event, () => this.playNext());
        break;
      case 'RewardGranted':
        this.animateReward(event, () => this.playNext());
        break;
      case 'LevelUp':
        this.animateLevelUp(event, () => this.playNext());
        break;
      case 'UnitDied':
        this.animateDeath(event, () => this.playNext());
        break;
      case 'UnitSpawned':
        this.animateSpawn(event, () => this.playNext());
        break;
      case 'BaseRegenerationApplied':
        this.animateBaseRegeneration(event as any, () => this.playNext());
        break;
      case 'ObjectiveDestroyed':
        this.animateObjectiveDestroyed(event as any, () => this.playNext());
        break;
      case 'HeroRespawned':
        this.animateHeroRespawned(event as any, () => this.playNext());
        break;
      case 'CoreDestroyed':
        this.animateCoreDestroyed(event as any, () => this.playNext());
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
    const duration = (250 * (points.length - 1)) / this.playbackSpeed;
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
    data: { target_id: number; damage: number; tower_id?: number; attacker_id?: number; target_hp_remaining?: number },
    onDone: () => void
  ): void {
    const targetSprite = this.renderer.getUnitSprite(data.target_id);
    const attackerId = data.tower_id ?? data.attacker_id;
    const attackerSprite = attackerId ? this.renderer.getUnitSprite(attackerId) : null;

    if (targetSprite) {
      if (data.tower_id && attackerSprite) {
        const beam = new PIXI.Graphics();
        beam.moveTo(attackerSprite.x, attackerSprite.y);
        beam.lineTo(targetSprite.x, targetSprite.y);
        beam.stroke({ color: 0xff3d00, width: 4, alpha: 0.9 });
        this.renderer.getFxLayer().addChild(beam);

        setTimeout(() => {
          this.renderer.getFxLayer().removeChild(beam);
        }, 180);
      } else if (attackerSprite && !data.tower_id) {
        const origX = attackerSprite.x;
        const origY = attackerSprite.y;
        const dx = targetSprite.x - origX;
        const dy = targetSprite.y - origY;
        const dist = Math.hypot(dx, dy) || 1;
        const lungeDist = 16;
        const lungeX = origX + (dx / dist) * lungeDist;
        const lungeY = origY + (dy / dist) * lungeDist;

        attackerSprite.x = lungeX;
        attackerSprite.y = lungeY;
        setTimeout(() => {
          attackerSprite.x = origX;
          attackerSprite.y = origY;
        }, 120);
      }

      if (data.target_hp_remaining !== undefined) {
        this.renderer.updateUnitHp(data.target_id, data.target_hp_remaining);
      }

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

  private animateSpellCast(
    data: { caster_id: number; spell_id: string; target: any },
    onDone: () => void
  ): void {
    const casterSprite = this.renderer.getUnitSprite(data.caster_id);
    if (!casterSprite) {
      onDone();
      return;
    }

    if (data.spell_id === 'cleave') {
      // Vanguard Cleave: Golden whirlwind shockwave ring (radius 1)
      const ring = new PIXI.Graphics();
      this.renderer.getFxLayer().addChild(ring);

      const startTime = performance.now();
      const duration = 280;

      const animate = () => {
        const elapsed = performance.now() - startTime;
        const progress = Math.min(elapsed / duration, 1);
        const radius = HEX_SIZE * (0.5 + progress * 1.2);

        ring.clear();
        ring.circle(casterSprite.x, casterSprite.y, radius);
        ring.stroke({ color: 0xffd600, width: 4 * (1 - progress), alpha: 0.9 * (1 - progress) });

        if (progress < 1) {
          requestAnimationFrame(animate);
        } else {
          this.renderer.getFxLayer().removeChild(ring);
          onDone();
        }
      };
      requestAnimationFrame(animate);
    } else if (data.spell_id === 'bolt') {
      // Ranger Bolt: Piercing high-voltage cyan laser railgun beam
      let targetPos: { x: number; y: number } | null = null;
      if (data.target?.payload?.unit_id) {
        const targetSprite = this.renderer.getUnitSprite(data.target.payload.unit_id);
        if (targetSprite) targetPos = { x: targetSprite.x, y: targetSprite.y };
      }

      if (targetPos) {
        const beam = new PIXI.Graphics();
        beam.moveTo(casterSprite.x, casterSprite.y);
        beam.lineTo(targetPos.x, targetPos.y);
        beam.stroke({ color: 0x00f5ff, width: 5, alpha: 0.95 });
        this.renderer.getFxLayer().addChild(beam);

        // Flash target
        const impact = new PIXI.Graphics();
        impact.circle(targetPos.x, targetPos.y, HEX_SIZE * 0.7);
        impact.fill({ color: 0x00f5ff, alpha: 0.6 });
        this.renderer.getFxLayer().addChild(impact);

        setTimeout(() => {
          this.renderer.getFxLayer().removeChild(beam);
          this.renderer.getFxLayer().removeChild(impact);
          onDone();
        }, 220);
      } else {
        onDone();
      }
    } else if (data.spell_id === 'mend') {
      // Warden Mend: Holy emerald healing sparkle pulse
      let targetSprite = casterSprite;
      if (data.target?.payload?.unit_id) {
        targetSprite = this.renderer.getUnitSprite(data.target.payload.unit_id) ?? casterSprite;
      }

      const healCircle = new PIXI.Graphics();
      this.renderer.getFxLayer().addChild(healCircle);

      const startTime = performance.now();
      const duration = 300;

      const animate = () => {
        const elapsed = performance.now() - startTime;
        const progress = Math.min(elapsed / duration, 1);

        healCircle.clear();
        healCircle.circle(targetSprite.x, targetSprite.y, HEX_SIZE * (0.3 + progress * 0.6));
        healCircle.stroke({ color: 0x10b981, width: 3, alpha: 1 - progress });
        healCircle.circle(targetSprite.x, targetSprite.y, HEX_SIZE * 0.2);
        healCircle.fill({ color: 0x34d399, alpha: 0.5 * (1 - progress) });

        if (progress < 1) {
          requestAnimationFrame(animate);
        } else {
          this.renderer.getFxLayer().removeChild(healCircle);
          onDone();
        }
      };
      requestAnimationFrame(animate);
    } else {
      onDone();
    }
  }

  private animateHeal(
    data: { caster_id: number; target_id: number; amount: number; target_hp_remaining?: number },
    onDone: () => void
  ): void {
    const targetSprite = this.renderer.getUnitSprite(data.target_id);
    if (targetSprite) {
      if (data.target_hp_remaining !== undefined) {
        this.renderer.updateUnitHp(data.target_id, data.target_hp_remaining);
      }
      this.showFloatingText(targetSprite.x, targetSprite.y, `+${data.amount} HP`, 0x10b981);
    }
    setTimeout(onDone, 160);
  }

  private animateRepair(
    data: { repairer_id: number; target_id: number; amount: number; target_hp_remaining?: number },
    onDone: () => void
  ): void {
    const targetSprite = this.renderer.getUnitSprite(data.target_id);
    if (targetSprite) {
      if (data.target_hp_remaining !== undefined) {
        this.renderer.updateUnitHp(data.target_id, data.target_hp_remaining);
      }
      this.showFloatingText(targetSprite.x, targetSprite.y, `+${data.amount} Repaired`, 0x38bdf8);
    }
    setTimeout(onDone, 180);
  }

  private animateTeamBuff(
    data: { team: number; buff_id: string; duration_rounds: number },
    onDone: () => void
  ): void {
    // Show objective buff notification banner
    this.showFloatingBanner(`🌟 CAMP BUFF: +5 ATK (${data.duration_rounds} RNDS)`, 0xf59e0b);
    setTimeout(onDone, 300);
  }

  private animateCampCleared(
    data: { camp_id: string; killer_team: number },
    onDone: () => void
  ): void {
    this.showFloatingBanner(`⚔️ ${data.camp_id.toUpperCase()} SECURED!`, 0x10b981);
    setTimeout(onDone, 250);
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
      sprite.scale.set(1 + progress * 0.3);

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.removeUnitSprite(data.unit_id);
        onDone();
      }
    };

    requestAnimationFrame(animate);
  }

  private animateSpawn(
    data: { unit_id: number; unit_kind: any; team: number; pos: HexCoord; spawner_id: number },
    onDone: () => void
  ): void {
    let sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite) {
      sprite = (this.renderer as any).createUnitSprite({
        id: data.unit_id,
        kind: data.unit_kind,
        team: data.team,
        pos: data.pos,
        hp: 40,
        max_hp: 40,
        ap: 1,
        max_ap: 1,
        initiative: 5,
        attack_damage: 8,
        attack_range: 1,
        vision_range: 2,
        spawn_counter: 0,
      });
      if (sprite) {
        this.renderer.getStage().addChild(sprite);
      }
    }

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
    this.showFloatingText(x, y, `-${damage}`, 0xff1744);
  }

  private showFloatingText(x: number, y: number, msg: string, color: number): void {
    const text = new PIXI.Text({
      text: msg,
      style: {
        fontSize: 16,
        fill: color,
        fontWeight: 'bold',
        stroke: { color: 0x000000, width: 3 },
      },
    });
    text.anchor.set(0.5);
    text.x = x;
    text.y = y - 25;
    this.renderer.getFxLayer().addChild(text);

    const startTime = performance.now();
    const duration = 650;

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      text.y = y - 25 - progress * 32;
      text.alpha = 1 - progress;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.getFxLayer().removeChild(text);
      }
    };

    requestAnimationFrame(animate);
  }

  private showFloatingBanner(msg: string, color: number): void {
    const app = this.renderer.getApp();
    const text = new PIXI.Text({
      text: msg,
      style: {
        fontSize: 20,
        fill: color,
        fontWeight: '900',
        stroke: { color: 0x000000, width: 4 },
        fontFamily: 'Outfit, sans-serif',
      },
    });
    text.anchor.set(0.5);
    text.x = app.screen.width / 2;
    text.y = app.screen.height / 2 - 80;
    this.renderer.getFxLayer().addChild(text);

    const startTime = performance.now();
    const duration = 1200;

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      text.y = app.screen.height / 2 - 80 - progress * 40;
      text.alpha = 1 - progress * 0.8;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.getFxLayer().removeChild(text);
      }
    };

    requestAnimationFrame(animate);
  }

  private animateReward(
    data: { unit_id: number; gold: number; xp: number; reason: string },
    onDone: () => void
  ): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite) {
      onDone();
      return;
    }

    let text = '';
    if (data.gold > 0 && data.xp > 0) {
      text = `+${data.gold}G  +${data.xp}XP`;
    } else if (data.gold > 0) {
      text = `+${data.gold}G`;
    } else if (data.xp > 0) {
      text = `+${data.xp}XP`;
    }

    if (text) {
      this.showFloatingText(sprite.x, sprite.y, text, 0xffd700);
    }
    setTimeout(onDone, 120);
  }

  private animateLevelUp(
    data: { unit_id: number; new_level: number },
    onDone: () => void
  ): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (!sprite) {
      onDone();
      return;
    }

    this.showFloatingText(
      sprite.x,
      sprite.y - 15,
      `★ LEVEL UP! LVL ${data.new_level} ★`,
      0xa855f7
    );

    const ring = new PIXI.Graphics();
    ring.circle(0, 0, 15);
    ring.stroke({ color: 0xffd700, width: 3 });
    ring.x = sprite.x;
    ring.y = sprite.y;
    this.renderer.getFxLayer().addChild(ring);

    const startTime = performance.now();
    const duration = 600;

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);

      ring.scale.set(1 + progress * 2.5);
      ring.alpha = 1 - progress;

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.getFxLayer().removeChild(ring);
        onDone();
      }
    };

    requestAnimationFrame(animate);
  }

  private createExpandingShockwave(
    x: number,
    y: number,
    color: number,
    maxRadius: number,
    duration: number,
    onComplete?: () => void
  ): void {
    const ring = new PIXI.Graphics();
    this.renderer.getFxLayer().addChild(ring);
    const startTime = performance.now();

    const animate = () => {
      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const radius = 10 + progress * maxRadius;

      ring.clear();
      ring.circle(x, y, radius);
      ring.stroke({ color, width: 4 * (1 - progress), alpha: 0.9 * (1 - progress) });

      if (progress < 1) {
        requestAnimationFrame(animate);
      } else {
        this.renderer.getFxLayer().removeChild(ring);
        onComplete?.();
      }
    };
    requestAnimationFrame(animate);
  }

  private animateBaseRegeneration(
    data: { unit_id: number; amount: number },
    onDone: () => void
  ): void {
    const sprite = this.renderer.getUnitSprite(data.unit_id);
    if (sprite) {
      this.showFloatingText(sprite.x, sprite.y, `+${data.amount} HP BASE REGEN`, 0x10b981);
    }
    setTimeout(onDone, 120);
  }

  private animateObjectiveDestroyed(
    data: { objective_id: number; gold_awarded_per_hero?: number; xp_awarded_per_hero?: number },
    onDone: () => void
  ): void {
    const gold = data.gold_awarded_per_hero ?? 50;
    const xp = data.xp_awarded_per_hero ?? 40;
    this.showFloatingBanner(`VAULT REWARD: +${gold}G +${xp}XP`, 0xffd700);
    const center = this.renderer.hexToPixel(0, 0);
    this.createExpandingShockwave(center.x, center.y, 0xffd700, 120, 500, onDone);
  }

  private animateHeroRespawned(
    data: { unit_id: number; pos?: HexCoord },
    onDone: () => void
  ): void {
    const pixelPos = data.pos
      ? this.renderer.hexToPixel(data.pos.q, data.pos.r)
      : this.renderer.getUnitSprite(data.unit_id);
    if (pixelPos) {
      this.showFloatingText(pixelPos.x, pixelPos.y, 'RESPAWNED!', 0x00f5ff);
      this.createExpandingShockwave(pixelPos.x, pixelPos.y, 0x00f5ff, 60, 400, onDone);
    } else {
      onDone();
    }
  }

  private animateCoreDestroyed(
    data: { core_id: number; team: number },
    onDone: () => void
  ): void {
    const sprite = this.renderer.getUnitSprite(data.core_id);
    const app = this.renderer.getApp();
    const x = sprite ? sprite.x : app.screen.width / 2;
    const y = sprite ? sprite.y : app.screen.height / 2;
    this.showFloatingBanner(`CORE DESTROYED!`, 0xff0055);
    this.createExpandingShockwave(x, y, 0xff0055, 180, 700, onDone);
  }
}
