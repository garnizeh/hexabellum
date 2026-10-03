import * as PIXI from 'pixi.js';
import { HexCoord, GameState, MoveTarget, UnitData, UnitKind } from './bridge';

export const HEX_SIZE = 30;

export class HexRenderer {
  private app: PIXI.Application;
  private hexLayer = new PIXI.Container();
  private overlayLayer = new PIXI.Container();
  private pathLayer = new PIXI.Container();
  private unitLayer = new PIXI.Container();
  private fogLayer = new PIXI.Container();
  private fxLayer = new PIXI.Container();

  private unitSprites = new Map<number, PIXI.Container>();
  private selectionRing = new PIXI.Graphics();
  private playerTeam: number = 0;

  constructor(app: PIXI.Application) {
    this.app = app;
    this.app.stage.addChild(this.hexLayer);
    this.app.stage.addChild(this.overlayLayer);
    this.app.stage.addChild(this.pathLayer);
    this.app.stage.addChild(this.unitLayer);
    this.app.stage.addChild(this.fogLayer);
    this.app.stage.addChild(this.fxLayer);

    this.overlayLayer.addChild(this.selectionRing);
  }

  setPlayerTeam(team: number): void {
    this.playerTeam = team;
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
        // Highlight central combat lane along r = 0
        const isLane = hex.r === 0;
        g.fill({ color: isLane ? 0x162438 : 0x101726 });
        g.stroke({ color: isLane ? 0x253b5c : 0x1a2638, width: 1 });
      }

      this.hexLayer.addChild(g);
    }
  }

  drawFog(visibleHexes: HexCoord[], allHexes: HexCoord[]): void {
    this.fogLayer.removeChildren();
    const visibleSet = new Set(visibleHexes.map(h => `${h.q},${h.r}`));

    for (const hex of allHexes) {
      if (!visibleSet.has(`${hex.q},${hex.r}`)) {
        const { x, y } = this.hexToPixel(hex.q, hex.r);
        const g = new PIXI.Graphics();
        const points: number[] = [];
        for (let i = 0; i < 6; i++) {
          const angle = (Math.PI / 3) * i - Math.PI / 6;
          points.push(x + (HEX_SIZE + 0.5) * Math.cos(angle), y + (HEX_SIZE + 0.5) * Math.sin(angle));
        }
        g.poly(points);
        g.fill({ color: 0x05070f, alpha: 0.78 });
        this.fogLayer.addChild(g);
      }
    }
  }

  drawUnits(state: GameState): void {
    this.unitLayer.removeChildren();
    this.unitSprites.clear();

    for (const [idStr, unit] of Object.entries(state.units)) {
      const id = Number(idStr);
      const sprite = this.createUnitSprite(unit);
      this.unitLayer.addChild(sprite);
      this.unitSprites.set(id, sprite);
    }
  }

  createUnitSprite(unit: UnitData): PIXI.Container {
    const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);

    const container = new PIXI.Container();
    container.x = x;
    container.y = y;

    const g = new PIXI.Graphics();
    const isFriendly = unit.team === this.playerTeam;
    const baseColor = unit.team === 0 ? 0x00d2ff : 0xff3366;

    // Draw stylized representation based on UnitKind
    switch (unit.kind) {
      case 'Hero': {
        g.circle(0, 0, HEX_SIZE * 0.52);
        g.fill({ color: baseColor });
        g.stroke({ color: isFriendly ? 0xffffff : 0xffb3c6, width: isFriendly ? 2.5 : 1.5 });
        // Inner core
        g.circle(0, 0, HEX_SIZE * 0.2);
        g.fill({ color: 0xffffff });
        break;
      }
      case 'Minion': {
        // Creep triangle pointing towards opponent base (Team 0 moves +x, Team 1 moves -x)
        const tip = unit.team === 0 ? HEX_SIZE * 0.45 : -HEX_SIZE * 0.45;
        g.poly([tip, 0, -tip * 0.7, -HEX_SIZE * 0.35, -tip * 0.7, HEX_SIZE * 0.35]);
        g.fill({ color: baseColor });
        g.stroke({ color: 0xffffff, width: 1.5 });
        break;
      }
      case 'Tower': {
        // Fortified turret square
        const size = HEX_SIZE * 0.8;
        g.rect(-size / 2, -size / 2, size, size);
        g.fill({ color: baseColor });
        g.stroke({ color: isFriendly ? 0xffffff : 0xffb3c6, width: 2.5 });
        // Inner core
        g.rect(-size / 4, -size / 4, size / 2, size / 2);
        g.fill({ color: 0x111118 });
        break;
      }
      case 'SpawnerTower': {
        // Spire crystal diamond
        g.poly([0, -HEX_SIZE * 0.6, HEX_SIZE * 0.5, 0, 0, HEX_SIZE * 0.6, -HEX_SIZE * 0.5, 0]);
        g.fill({ color: isFriendly ? 0x7c4dff : 0xff9100 });
        g.stroke({ color: 0xffffff, width: 2 });
        break;
      }
      default:
        g.circle(0, 0, HEX_SIZE * 0.4);
        g.fill({ color: 0x888888 });
        break;
    }

    // Spawner wave timer dots or Hero AP dots
    if (unit.kind === 'SpawnerTower' && unit.spawn_interval) {
      const dotsY = HEX_SIZE * 0.75;
      for (let i = 0; i < unit.spawn_interval; i++) {
        const dotX = (i - (unit.spawn_interval - 1) / 2) * 10;
        g.circle(dotX, dotsY, 3);
        g.fill({ color: i < unit.spawn_counter ? 0x7c4dff : 0x333344 });
      }
    } else if (unit.kind === 'Hero') {
      const apY = HEX_SIZE * 0.75;
      for (let i = 0; i < unit.max_ap; i++) {
        const apX = (i - (unit.max_ap - 1) / 2) * 10;
        g.circle(apX, apY, 3);
        g.fill({ color: i < unit.ap ? 0xffd600 : 0x333344 });
      }

      // Initiative badge
      const initText = new PIXI.Text({
        text: `⚡${unit.initiative}`,
        style: {
          fontSize: 9,
          fill: 0xffffff,
          fontFamily: 'Outfit, sans-serif',
          fontWeight: 'bold',
        },
      });
      initText.anchor.set(0.5);
      initText.y = 0;
      container.addChild(initText);
    }

    container.addChild(g);

    // Dedicated HP Bar Graphic
    const hpBar = new PIXI.Graphics();
    this.renderHpBar(hpBar, unit.hp, unit.max_hp);
    container.addChild(hpBar);
    (container as any).hpBar = hpBar;
    (container as any).maxHp = unit.max_hp;

    return container;
  }

  private renderHpBar(g: PIXI.Graphics, hp: number, maxHp: number): void {
    g.clear();
    const hpWidth = HEX_SIZE * 0.9;
    const hpHeight = 5;
    const hpX = -hpWidth / 2;
    const hpY = -HEX_SIZE * 0.8;
    const hpRatio = Math.max(0, Math.min(1, hp / maxHp));

    g.rect(hpX, hpY, hpWidth, hpHeight);
    g.fill({ color: 0x111118 });
    if (hpRatio > 0) {
      g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
      g.fill({ color: hpRatio > 0.5 ? 0x00e676 : hpRatio > 0.25 ? 0xffea00 : 0xff1744 });
    }
  }

  updateUnitHp(unitId: number, hp: number, maxHp?: number): void {
    const sprite = this.unitSprites.get(unitId);
    if (!sprite) return;
    const hpBar = (sprite as any).hpBar as PIXI.Graphics;
    const mHp = maxHp ?? (sprite as any).maxHp ?? 100;
    if (hpBar) {
      this.renderHpBar(hpBar, hp, mHp);
    }
  }

  spawnUnit(event: { unit_id: number; unit_kind: UnitKind; team: number; pos: HexCoord; spawner_id: number }): PIXI.Container {
    const isMinion = event.unit_kind === 'Minion';
    const unit: UnitData = {
      id: event.unit_id,
      kind: event.unit_kind,
      team: event.team,
      pos: event.pos,
      hp: isMinion ? 30 : 100,
      max_hp: isMinion ? 30 : 100,
      ap: isMinion ? 2 : 3,
      max_ap: isMinion ? 2 : 3,
      initiative: isMinion ? 1 : 2,
      attack_damage: isMinion ? 8 : 20,
      attack_range: 1,
      vision_range: isMinion ? 2 : 3,
      spawn_counter: 0,
    };
    const sprite = this.createUnitSprite(unit);
    this.unitLayer.addChild(sprite);
    this.unitSprites.set(event.unit_id, sprite);
    return sprite;
  }

  drawMoveTargets(targets: (MoveTarget | HexCoord)[]): void {
    this.overlayLayer.removeChildren();
    this.overlayLayer.addChild(this.selectionRing);
    for (const target of targets) {
      const { x, y } = this.hexToPixel(target.q, target.r);
      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.35);
      g.fill({ color: 0x00e676, alpha: 0.4 });
      g.stroke({ color: 0x00e676, width: 2 });

      if ('cost' in target) {
        const costText = new PIXI.Text({
          text: `${target.cost}`,
          style: {
            fontSize: 10,
            fill: 0xffffff,
            fontFamily: 'Outfit, sans-serif',
            fontWeight: 'bold',
          },
        });
        costText.anchor.set(0.5);
        costText.x = x;
        costText.y = y;
        this.overlayLayer.addChild(costText);
      }

      this.overlayLayer.addChild(g);
    }
  }

  drawAttackTargets(targetIds: number[], state: GameState): void {
    for (const targetId of targetIds) {
      const unit = state.units[targetId];
      if (!unit) continue;

      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);
      const g = new PIXI.Graphics();
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
    this.selectionRing.clear();
    if (unitId === null) return;

    const sprite = this.unitSprites.get(unitId);
    if (sprite) {
      this.selectionRing.circle(sprite.x, sprite.y, HEX_SIZE * 0.68);
      this.selectionRing.stroke({ color: 0xffd600, width: 2.5, alpha: 0.95 });
      this.selectionRing.circle(sprite.x, sprite.y, HEX_SIZE * 0.78);
      this.selectionRing.stroke({ color: 0xffd600, width: 1.5, alpha: 0.45 });
    }
  }

  clearMoveTargets(): void {
    this.overlayLayer.removeChildren();
    this.overlayLayer.addChild(this.selectionRing);
  }

  clearOverlays(): void {
    this.overlayLayer.removeChildren();
    this.overlayLayer.addChild(this.selectionRing);
    this.selectionRing.clear();
    this.pathLayer.removeChildren();
  }

  getUnitSprite(unitId: number): PIXI.Container | undefined {
    return this.unitSprites.get(unitId);
  }

  removeUnitSprite(unitId: number): void {
    const sprite = this.unitSprites.get(unitId);
    if (sprite) {
      this.unitLayer.removeChild(sprite);
      this.unitSprites.delete(unitId);
    }
  }

  getStage(): PIXI.Container {
    return this.app.stage;
  }

  getFxLayer(): PIXI.Container {
    return this.fxLayer;
  }

  getApp(): PIXI.Application {
    return this.app;
  }
}
