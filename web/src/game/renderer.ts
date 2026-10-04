import * as PIXI from 'pixi.js';
import { HexCoord, GameState, MoveTarget, UnitData, UnitKind } from './bridge';
import { BaseZoneDto } from './types';
import { CameraController } from './camera';

export const HEX_SIZE = 30;

export class HexRenderer {
  private app: PIXI.Application;
  private worldContainer = new PIXI.Container();

  // 11-Layer WebGL Rendering Hierarchy (aligned with docs/ui-ux.md §4.6)
  private voidLayer = new PIXI.Container();          // Layer 1: Arena Void Canvas
  private hexLayer = new PIXI.Container();           // Layer 2: Base Hex Grid
  private baseZoneLayer = new PIXI.Container();      // Layer 3: Base Zone Sanctuaries
  private obstacleLayer = new PIXI.Container();      // Layer 4: Terrain Obstacles & Line-of-Sight Walls
  private fogLayer = new PIXI.Container();           // Layer 5: Fog of War Shroud
  private overlayLayer = new PIXI.Container();       // Layer 6: Targeting & Range Highlights
  private pathLayer = new PIXI.Container();          // Layer 7: Order Splines
  private ghostLayer = new PIXI.Container();         // Layer 7: Ghost Projections
  private unitLayer = new PIXI.Container();          // Layer 8: Dynamic Entities
  private entityOverlayLayer = new PIXI.Container(); // Layer 9: Entity Overlays
  private vfxLayer = new PIXI.Container();           // Layer 10: VFX & Particles
  private fctLayer = new PIXI.Container();           // Layer 11: UI Reticles & Combat Floating Text

  // Backward compatibility alias
  public get fxLayer(): PIXI.Container { return this.vfxLayer; }

  private unitSprites = new Map<number, PIXI.Container>();
  private selectionRing = new PIXI.Graphics();
  private raycastLine = new PIXI.Graphics();
  private playerTeam: number = 0;
  private camera: CameraController;

  constructor(app: PIXI.Application) {
    this.app = app;

    // Attach world container to stage to allow unified pan/zoom
    this.app.stage.addChild(this.worldContainer);

    // Structure 11 layers in strict z-index order
    this.worldContainer.addChild(this.voidLayer);
    this.worldContainer.addChild(this.hexLayer);
    this.worldContainer.addChild(this.baseZoneLayer);
    this.worldContainer.addChild(this.obstacleLayer);
    this.worldContainer.addChild(this.fogLayer);
    this.worldContainer.addChild(this.overlayLayer);
    this.worldContainer.addChild(this.pathLayer);
    this.worldContainer.addChild(this.ghostLayer);
    this.worldContainer.addChild(this.unitLayer);
    this.worldContainer.addChild(this.entityOverlayLayer);
    this.worldContainer.addChild(this.vfxLayer);
    this.worldContainer.addChild(this.fctLayer);

    this.overlayLayer.addChild(this.selectionRing);
    this.overlayLayer.addChild(this.raycastLine);

    // Initial position: center world at screen center
    const sw = this.app.screen.width;
    const sh = this.app.screen.height;
    this.worldContainer.position.set(sw / 2, sh / 2);

    this.camera = new CameraController(
      this.worldContainer,
      this.app.canvas as HTMLCanvasElement
    );
  }

  getCamera(): CameraController {
    return this.camera;
  }

  setPlayerTeam(team: number): void {
    this.playerTeam = team;
  }

  /**
   * Converts axial hex coordinates (q, r) to local world pixel coordinates inside worldContainer.
   */
  hexToPixel(q: number, r: number): { x: number; y: number } {
    const x = HEX_SIZE * (Math.sqrt(3) * q + (Math.sqrt(3) / 2) * r);
    const y = HEX_SIZE * (3 / 2) * r;
    return { x, y };
  }

  /**
   * Converts screen (canvas client) coordinates to worldContainer coordinates considering zoom and pan.
   */
  screenToWorld(screenX: number, screenY: number): { x: number; y: number } {
    const zoom = this.worldContainer.scale.x;
    return {
      x: (screenX - this.worldContainer.x) / zoom,
      y: (screenY - this.worldContainer.y) / zoom,
    };
  }

  /**
   * Converts world coordinates to rounded axial hex coordinate (q, r).
   */
  worldToHex(worldX: number, worldY: number): { q: number; r: number } {
    const q = ((Math.sqrt(3) / 3) * worldX - (1 / 3) * worldY) / HEX_SIZE;
    const r = ((2 / 3) * worldY) / HEX_SIZE;
    return this.hexRound(q, r);
  }

  /**
   * Converts screen (canvas client) coordinates to rounded axial hex coordinate (q, r).
   */
  screenToHex(screenX: number, screenY: number): { q: number; r: number } {
    const w = this.screenToWorld(screenX, screenY);
    return this.worldToHex(w.x, w.y);
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

  centerOnHex(q: number, r: number): void {
    const pos = this.hexToPixel(q, r);
    this.camera.centerOn(
      pos.x,
      pos.y,
      this.app.screen.width,
      this.app.screen.height,
      true
    );
  }

  drawMap(hexes: HexCoord[], obstacles: HexCoord[]): void {
    this.hexLayer.removeChildren();
    this.obstacleLayer.removeChildren();
    this.voidLayer.removeChildren();

    // Layer 1: Arena Void Canvas (starry nebula backdrop & boundary per docs/ui-ux.md §4.6)
    const voidG = new PIXI.Graphics();
    const arenaRadius = HEX_SIZE * 8 * 1.75;
    voidG.circle(0, 0, arenaRadius + 60);
    voidG.fill({ color: 0x070a13, alpha: 0.98 });
    voidG.stroke({ color: 0x1e293b, width: 2, alpha: 0.6 });
    for (let i = 0; i < 70; i++) {
      const angle = (i * 137.5 * Math.PI) / 180;
      const dist = (arenaRadius * 0.25) + ((i * 37) % (arenaRadius * 0.9));
      const sx = dist * Math.cos(angle);
      const sy = dist * Math.sin(angle);
      voidG.circle(sx, sy, (i % 3 === 0) ? 1.5 : 1);
      voidG.fill({ color: 0xffffff, alpha: (i % 2 === 0) ? 0.35 : 0.18 });
    }
    this.voidLayer.addChild(voidG);
    const obstacleSet = new Set(obstacles.map(h => `${h.q},${h.r}`));

    // Known terrain positions (Radius 8 arena features)
    const walls = new Set(['0,2', '0,-2', '-2,-3', '2,-3', '-3,2', '3,-2']);
    const smokePillars = new Set(['2,2', '-2,-2']);
    const boulders = new Set(['0,1', '0,-1']);
    const shrines = new Set(['0,4', '0,-4', '0,3', '0,-3']);

    for (const hex of hexes) {
      const { x, y } = this.hexToPixel(hex.q, hex.r);
      const hexKey = `${hex.q},${hex.r}`;
      const isObstacle = obstacleSet.has(hexKey);

      const g = new PIXI.Graphics();
      const points: number[] = [];
      for (let i = 0; i < 6; i++) {
        const angle = (Math.PI / 3) * i - Math.PI / 6;
        points.push(x + HEX_SIZE * Math.cos(angle), y + HEX_SIZE * Math.sin(angle));
      }
      g.poly(points);

      if (shrines.has(hexKey)) {
        // Ancient neutral camp shrine
        g.fill({ color: 0x064e3b, alpha: 0.85 });
        g.stroke({ color: 0x10b981, width: 2 });
        // Emerald rune ring
        g.circle(x, y, HEX_SIZE * 0.6);
        g.stroke({ color: 0x34d399, width: 1.5, alpha: 0.7 });
        g.circle(x, y, HEX_SIZE * 0.25);
        g.fill({ color: 0x10b981, alpha: 0.5 });
      } else if (walls.has(hexKey)) {
        // Dense stone wall (blocks move & blocks sight)
        g.fill({ color: 0x1e293b });
        g.stroke({ color: 0x475569, width: 2.5 });
        // Stone battlement hatching
        g.rect(x - HEX_SIZE * 0.4, y - HEX_SIZE * 0.4, HEX_SIZE * 0.8, HEX_SIZE * 0.8);
        g.fill({ color: 0x334155 });
        g.stroke({ color: 0x64748b, width: 1 });
      } else if (smokePillars.has(hexKey)) {
        // Smoke pillar (blocks sight only, allows move)
        g.fill({ color: 0x2e1065, alpha: 0.8 });
        g.stroke({ color: 0x7c3aed, width: 2 });
        // Swirling misty cloud rings
        g.circle(x, y, HEX_SIZE * 0.65);
        g.stroke({ color: 0xa78bfa, width: 1.5, alpha: 0.6 });
        g.circle(x, y, HEX_SIZE * 0.35);
        g.fill({ color: 0x8b5cf6, alpha: 0.35 });
      } else if (boulders.has(hexKey)) {
        // Low boulders (blocks move only, allows sight)
        g.fill({ color: 0x292524 });
        g.stroke({ color: 0x78716c, width: 2 });
        // Jagged rock cluster
        g.circle(x - 5, y - 3, HEX_SIZE * 0.28);
        g.fill({ color: 0x44403c });
        g.circle(x + 5, y + 4, HEX_SIZE * 0.32);
        g.fill({ color: 0x57534e });
      } else if (isObstacle) {
        g.fill({ color: 0x24243a });
        g.stroke({ color: 0x3d3d5c, width: 1.5 });
      } else {
        // Highlight central combat lane along r = 0
        const isLane = hex.r === 0;
        g.fill({ color: isLane ? 0x162438 : 0x101726 });
        g.stroke({ color: isLane ? 0x253b5c : 0x1a2638, width: 1 });
        if (isLane && Math.abs(hex.q) <= 6) {
          // Subtle lane center path dot
          g.circle(x, y, 2.5);
          g.fill({ color: 0x38bdf8, alpha: 0.35 });
        }
      }

      this.hexLayer.addChild(g);
    }
  }

  drawBaseZones(baseZones: BaseZoneDto[]): void {
    this.baseZoneLayer.removeChildren();

    for (const bz of baseZones) {
      const isAlly = bz.team === this.playerTeam;
      const fillColor = isAlly ? 0x22d3ee : 0xf43f5e;
      const strokeColor = isAlly ? 0x38bdf8 : 0xfb7185;

      // Draw all hexes within base radius (r=2, 19 hexes)
      for (let dq = -bz.radius; dq <= bz.radius; dq++) {
        for (
          let dr = Math.max(-bz.radius, -dq - bz.radius);
          dr <= Math.min(bz.radius, -dq + bz.radius);
          dr++
        ) {
          const q = bz.center.q + dq;
          const r = bz.center.r + dr;
          const { x, y } = this.hexToPixel(q, r);

          const g = new PIXI.Graphics();
          const points: number[] = [];
          for (let i = 0; i < 6; i++) {
            const angle = (Math.PI / 3) * i - Math.PI / 6;
            points.push(x + HEX_SIZE * Math.cos(angle), y + HEX_SIZE * Math.sin(angle));
          }
          g.poly(points);
          g.fill({ color: fillColor, alpha: 0.12 });
          g.stroke({ color: strokeColor, width: 1.5, alpha: 0.45 });
          this.baseZoneLayer.addChild(g);
        }
      }

      // Celestial beacon rings at Core center hex
      const centerPos = this.hexToPixel(bz.center.q, bz.center.r);
      const beacon = new PIXI.Graphics();
      beacon.circle(centerPos.x, centerPos.y, HEX_SIZE * 0.85);
      beacon.stroke({ color: strokeColor, width: 2, alpha: 0.65 });
      beacon.circle(centerPos.x, centerPos.y, HEX_SIZE * 1.15);
      beacon.stroke({ color: strokeColor, width: 1, alpha: 0.3 });
      this.baseZoneLayer.addChild(beacon);
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
      if (
        unit.life_state === 'dead_awaiting_respawn' ||
        unit.life_state === 'permanently_removed' ||
        unit.hp === 0
      ) {
        continue;
      }
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
    const isNeutral = unit.team === 255 || unit.kind === 'Neutral' || unit.kind === 'NeutralGuardian';
    const baseColor = isNeutral ? 0xf59e0b : unit.team === 0 ? 0x00d2ff : 0xff3366;

    // Draw active team buff / status glow halo
    if (unit.statuses && unit.statuses.length > 0) {
      const halo = new PIXI.Graphics();
      halo.circle(0, 0, HEX_SIZE * 0.72);
      halo.stroke({ color: 0x10b981, width: 2, alpha: 0.85 });
      halo.circle(0, 0, HEX_SIZE * 0.84);
      halo.stroke({ color: 0x34d399, width: 1, alpha: 0.4 });
      container.addChild(halo);
    }

    // Stylized representation based on UnitKind
    switch (unit.kind) {
      case 'Neutral':
      case 'NeutralGuardian': {
        // Ancient Runic Stone Guardian
        g.poly([
          0, -HEX_SIZE * 0.65,
          HEX_SIZE * 0.55, -HEX_SIZE * 0.25,
          HEX_SIZE * 0.45, HEX_SIZE * 0.45,
          -HEX_SIZE * 0.45, HEX_SIZE * 0.45,
          -HEX_SIZE * 0.55, -HEX_SIZE * 0.25,
        ]);
        g.fill({ color: 0xd97706 });
        g.stroke({ color: 0xfef08a, width: 2 });
        // Golden core eye
        g.circle(0, 0, HEX_SIZE * 0.24);
        g.fill({ color: 0xfef08a });
        break;
      }
      case 'Hero': {
        g.circle(0, 0, HEX_SIZE * 0.54);
        g.fill({ color: baseColor });
        g.stroke({ color: isFriendly ? 0xffffff : 0xffb3c6, width: isFriendly ? 2.5 : 1.5 });

        // Archetype specific insignia
        if (unit.max_hp === 140 || unit.cooldowns?.cleave !== undefined) {
          // Vanguard: Frontline Shield
          g.poly([0, -8, 7, -3, 5, 6, 0, 9, -5, 6, -7, -3]);
          g.fill({ color: 0xffffff });
        } else if (unit.cooldowns?.longshot !== undefined || unit.attack_range >= 3 || unit.max_hp === 80) {
          // Sniper: Precision Crosshair Reticle with Laser Dot
          g.circle(0, 0, 7.5);
          g.stroke({ color: 0xffffff, width: 1.5 });
          g.moveTo(-10, 0); g.lineTo(10, 0); g.stroke({ color: 0xffffff, width: 1.5 });
          g.moveTo(0, -10); g.lineTo(0, 10); g.stroke({ color: 0xffffff, width: 1.5 });
          g.circle(0, 0, 1.8);
          g.fill({ color: 0xff1744 });
        } else if (unit.cooldowns?.fury !== undefined || (unit.max_hp === 120 && unit.attack_range === 1)) {
          // Berserker: Crossed Rage Blades
          g.poly([-6, -7, -4, -8, 6, 7, 4, 8]); g.fill({ color: 0xffffff });
          g.poly([6, -7, 4, -8, -6, 7, -4, 8]); g.fill({ color: 0xffffff });
          g.circle(0, 0, 2.5); g.fill({ color: 0xff3366 });
        } else if (unit.attack_range >= 2 || unit.cooldowns?.bolt !== undefined) {
          // Ranger: Crosshair
          g.circle(0, 0, 7);
          g.stroke({ color: 0xffffff, width: 2 });
          g.circle(0, 0, 2);
          g.fill({ color: 0xffffff });
        } else if (unit.max_energy === 6 || unit.cooldowns?.mend !== undefined) {
          // Warden: Medic Cross
          g.rect(-2.5, -8, 5, 16);
          g.rect(-8, -2.5, 16, 5);
          g.fill({ color: 0xffffff });
        } else {
          g.circle(0, 0, HEX_SIZE * 0.2);
          g.fill({ color: 0xffffff });
        }
        break;
      }
      case 'Minion': {
        // Creep triangle pointing towards opponent base
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
        g.rect(-size / 4, -size / 4, size / 2, size / 2);
        g.fill({ color: 0x111118 });
        break;
      }
      case 'Spawner':
      case 'SpawnerTower': {
        // Spire crystal diamond
        g.poly([0, -HEX_SIZE * 0.6, HEX_SIZE * 0.5, 0, 0, HEX_SIZE * 0.6, -HEX_SIZE * 0.5, 0]);
        g.fill({ color: isFriendly ? 0x7c4dff : 0xff9100 });
        g.stroke({ color: 0xffffff, width: 2 });
        break;
      }
      case 'Core': {
        // Monolithic towering crystal pulsing with team energy (2.5x scale)
        const crystalColor = unit.team === 0 ? 0x00d2ff : 0xff3366;
        const crystalGlow = unit.team === 0 ? 0x38bdf8 : 0xfb7185;

        // Base pedestal
        g.ellipse(0, HEX_SIZE * 0.75, HEX_SIZE * 0.9, HEX_SIZE * 0.35);
        g.fill({ color: 0x0f172a, alpha: 0.9 });
        g.stroke({ color: crystalColor, width: 2, alpha: 0.8 });

        // Outer crystal prism (towering 2.5x height)
        g.poly([
          0, -HEX_SIZE * 1.45,
          HEX_SIZE * 0.75, -HEX_SIZE * 0.3,
          HEX_SIZE * 0.55, HEX_SIZE * 0.75,
          0, HEX_SIZE * 0.95,
          -HEX_SIZE * 0.55, HEX_SIZE * 0.75,
          -HEX_SIZE * 0.75, -HEX_SIZE * 0.3,
        ]);
        g.fill({ color: crystalColor, alpha: 0.85 });
        g.stroke({ color: 0xffffff, width: 2.5 });

        // Inner core facets & highlights
        g.poly([
          0, -HEX_SIZE * 1.25,
          HEX_SIZE * 0.4, -HEX_SIZE * 0.2,
          0, HEX_SIZE * 0.65,
          -HEX_SIZE * 0.4, -HEX_SIZE * 0.2,
        ]);
        g.fill({ color: 0xffffff, alpha: 0.4 });

        // Glowing heart
        g.circle(0, 0, HEX_SIZE * 0.25);
        g.fill({ color: 0xffffff, alpha: 0.9 });
        g.stroke({ color: crystalGlow, width: 2 });
        break;
      }
      case 'Objective': {
        // Ancient Runic Stone Obelisk at (0, 0)
        // Stepped slate pedestal
        g.rect(-HEX_SIZE * 0.7, HEX_SIZE * 0.35, HEX_SIZE * 1.4, HEX_SIZE * 0.4);
        g.fill({ color: 0x0f172a });
        g.stroke({ color: 0xd97706, width: 2 });

        // Tapered ancient obelisk
        g.poly([
          -HEX_SIZE * 0.5, HEX_SIZE * 0.4,
          HEX_SIZE * 0.5, HEX_SIZE * 0.4,
          HEX_SIZE * 0.35, -HEX_SIZE * 0.9,
          0, -HEX_SIZE * 1.35,
          -HEX_SIZE * 0.35, -HEX_SIZE * 0.9,
        ]);
        g.fill({ color: 0x1e293b });
        g.stroke({ color: 0xf59e0b, width: 2.5 });

        // Amber mystical runes & carvings
        // Central rune eye
        g.circle(0, -HEX_SIZE * 0.15, HEX_SIZE * 0.22);
        g.fill({ color: 0xf59e0b, alpha: 0.85 });
        g.stroke({ color: 0xfef08a, width: 1.5 });
        g.circle(0, -HEX_SIZE * 0.15, HEX_SIZE * 0.08);
        g.fill({ color: 0xffffff });

        // Runic diamond glyphs
        g.poly([0, -HEX_SIZE * 0.65, 6, -HEX_SIZE * 0.5, 0, -HEX_SIZE * 0.35, -6, -HEX_SIZE * 0.5]);
        g.fill({ color: 0xfbbf24, alpha: 0.9 });

        g.poly([0, HEX_SIZE * 0.05, 5, HEX_SIZE * 0.18, 0, HEX_SIZE * 0.3, -5, HEX_SIZE * 0.18]);
        g.fill({ color: 0xfbbf24, alpha: 0.9 });
        break;
      }
      default:
        g.circle(0, 0, HEX_SIZE * 0.4);
        g.fill({ color: 0x888888 });
        break;
    }

    // Hero Resource Indicators: AP (Yellow Dots) & Energy (Cyan Blue Pips)
    if (unit.kind === 'Hero') {
      const apY = HEX_SIZE * 0.72;
      for (let i = 0; i < unit.max_ap; i++) {
        const apX = (i - (unit.max_ap - 1) / 2) * 9;
        g.circle(apX, apY, 2.5);
        g.fill({ color: i < unit.ap ? 0xffd600 : 0x333344 });
      }

      const energy = unit.energy ?? 0;
      const maxEnergy = unit.max_energy ?? 5;
      const enY = HEX_SIZE * 0.88;
      for (let i = 0; i < maxEnergy; i++) {
        const enX = (i - (maxEnergy - 1) / 2) * 7;
        g.circle(enX, enY, 2);
        g.fill({ color: i < energy ? 0x00f5ff : 0x1e293b });
      }

      // Initiative badge
      const initText = new PIXI.Text({
        text: `⚡${unit.initiative}`,
        style: {
          fontSize: 8.5,
          fill: 0xffffff,
          fontFamily: 'Outfit, sans-serif',
          fontWeight: 'bold',
        },
      });
      initText.anchor.set(0.5);
      initText.y = -HEX_SIZE * 0.5;
      container.addChild(initText);
    } else if ((unit.kind === 'Spawner' || unit.kind === 'SpawnerTower') && unit.spawn_interval) {
      const dotsY = HEX_SIZE * 0.75;
      for (let i = 0; i < unit.spawn_interval; i++) {
        const dotX = (i - (unit.spawn_interval - 1) / 2) * 10;
        g.circle(dotX, dotsY, 3);
        g.fill({ color: i < unit.spawn_counter ? 0x7c4dff : 0x333344 });
      }
    }

    container.addChild(g);

    // Dedicated HP Bar Graphic
    const hpBar = new PIXI.Graphics();
    this.renderHpBar(hpBar, unit.hp, unit.max_hp, unit);
    container.addChild(hpBar);
    (container as any).hpBar = hpBar;
    (container as any).maxHp = unit.max_hp;
    (container as any).unit = unit;

    return container;
  }

  private renderHpBar(g: PIXI.Graphics, hp: number, maxHp: number, unit?: UnitData): void {
    g.clear();
    const kind = unit?.kind;

    if (kind === 'Core') {
      // Large segmented bar (10 segments of 70 HP each) displaying current and max HP
      const hpWidth = HEX_SIZE * 2.2;
      const hpHeight = 9;
      const hpX = -hpWidth / 2;
      const hpY = -HEX_SIZE * 1.75;
      const segments = 10;
      const segWidth = (hpWidth - (segments - 1) * 1.5) / segments;
      const hpPerSeg = maxHp / segments;

      // Dark background panel
      g.rect(hpX - 3, hpY - 3, hpWidth + 6, hpHeight + 6);
      g.fill({ color: 0x090b14, alpha: 0.95 });
      g.stroke({ color: unit?.team === 0 ? 0x00d2ff : 0xff3366, width: 1.5 });

      for (let i = 0; i < segments; i++) {
        const segX = hpX + i * (segWidth + 1.5);
        const segMinHp = i * hpPerSeg;

        // Base slot
        g.rect(segX, hpY, segWidth, hpHeight);
        g.fill({ color: 0x1e293b });

        if (hp > segMinHp) {
          const segRatio = Math.min(1, Math.max(0, (hp - segMinHp) / hpPerSeg));
          g.rect(segX, hpY, segWidth * segRatio, hpHeight);
          const color = unit?.team === 0 ? 0x00d2ff : 0xff3366;
          g.fill({ color });
        }
      }
      return;
    }

    if (kind === 'Objective') {
      // Ancient Vault Amber HP bar
      const hpWidth = HEX_SIZE * 1.6;
      const hpHeight = 7;
      const hpX = -hpWidth / 2;
      const hpY = -HEX_SIZE * 1.55;
      const hpRatio = Math.max(0, Math.min(1, hp / maxHp));

      g.rect(hpX - 2, hpY - 2, hpWidth + 4, hpHeight + 4);
      g.fill({ color: 0x0f172a, alpha: 0.9 });
      g.stroke({ color: 0xf59e0b, width: 1.5 });

      g.rect(hpX, hpY, hpWidth, hpHeight);
      g.fill({ color: 0x292524 });

      if (hpRatio > 0) {
        g.rect(hpX, hpY, hpWidth * hpRatio, hpHeight);
        g.fill({ color: 0xf59e0b });
      }
      return;
    }

    // Standard Unit HP bar
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
    const unit = (sprite as any).unit as UnitData | undefined;
    if (hpBar) {
      this.renderHpBar(hpBar, hp, mHp, unit);
    }
  }

  drawMoveTargets(targets: (MoveTarget | HexCoord)[]): void {
    this.clearOverlays();
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

  drawAbilityTargets(validUnitIds: number[], obstructedUnitIds: number[], state: GameState): void {
    // Valid targets in Cyan
    for (const targetId of validUnitIds) {
      const unit = state.units[targetId];
      if (!unit) continue;
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);
      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.68);
      g.stroke({ color: 0x00f5ff, width: 3, alpha: 0.95 });
      g.circle(x, y, HEX_SIZE * 0.3);
      g.stroke({ color: 0x00f5ff, width: 1.5, alpha: 0.6 });
      this.overlayLayer.addChild(g);
    }

    // Obstructed targets in Red with slash indicator
    for (const targetId of obstructedUnitIds) {
      const unit = state.units[targetId];
      if (!unit) continue;
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);
      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.68);
      g.stroke({ color: 0xff1744, width: 2.5, alpha: 0.8 });
      // Red obstruction slash
      g.moveTo(x - HEX_SIZE * 0.45, y - HEX_SIZE * 0.45);
      g.lineTo(x + HEX_SIZE * 0.45, y + HEX_SIZE * 0.45);
      g.stroke({ color: 0xff1744, width: 2.5, alpha: 0.85 });
      this.overlayLayer.addChild(g);
    }
  }

  drawRepairTargets(targetIds: number[], state: GameState): void {
    for (const targetId of targetIds) {
      const unit = state.units[targetId];
      if (!unit) continue;
      const { x, y } = this.hexToPixel(unit.pos.q, unit.pos.r);
      const g = new PIXI.Graphics();
      g.circle(x, y, HEX_SIZE * 0.72);
      g.stroke({ color: 0x00e676, width: 3, alpha: 0.95 });
      // Repair cross/wrench badge
      g.rect(x - 2, y - 8, 4, 16);
      g.rect(x - 8, y - 2, 16, 4);
      g.fill({ color: 0x00e676, alpha: 0.8 });
      this.overlayLayer.addChild(g);
    }
  }

  drawRaycastLine(from: HexCoord, to: HexCoord, isBlocked: boolean): void {
    this.raycastLine.clear();
    const p1 = this.hexToPixel(from.q, from.r);
    const p2 = this.hexToPixel(to.q, to.r);

    this.raycastLine.moveTo(p1.x, p1.y);
    this.raycastLine.lineTo(p2.x, p2.y);
    if (isBlocked) {
      this.raycastLine.stroke({ color: 0xff1744, width: 2, alpha: 0.85 });
    } else {
      this.raycastLine.stroke({ color: 0x00f5ff, width: 2, alpha: 0.85 });
    }
  }

  clearRaycastLine(): void {
    this.raycastLine.clear();
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
    g.stroke({ color: 0x38bdf8, width: 2.5, alpha: 0.85 });
    this.pathLayer.addChild(g);
  }

  /**
   * Draws a semi-transparent Ghost Hero projection at the drafted destination tile
   * with a directional connecting spline (aligned with docs/ui-ux.md §4.8 & §4.9).
   */
  drawGhostHero(unit: UnitData, destination: HexCoord): void {
    this.clearGhost();

    const destPixel = this.hexToPixel(destination.q, destination.r);
    const startPixel = this.hexToPixel(unit.pos.q, unit.pos.r);

    const ghostContainer = new PIXI.Container();
    ghostContainer.x = destPixel.x;
    ghostContainer.y = destPixel.y;
    ghostContainer.alpha = 0.65;

    const isFriendly = unit.team === this.playerTeam;
    const glowColor = isFriendly ? 0x38bdf8 : 0xf87171;

    const g = new PIXI.Graphics();
    // Destination pulse ring
    g.circle(0, 0, HEX_SIZE * 0.68);
    g.stroke({ color: glowColor, width: 2, alpha: 0.9 });
    g.circle(0, 0, HEX_SIZE * 0.54);
    g.fill({ color: glowColor, alpha: 0.35 });
    g.stroke({ color: 0xffffff, width: 1.5, alpha: 0.8 });

    // Archetype silhouette icon inside ghost
    if (unit.max_hp === 140 || unit.cooldowns?.cleave !== undefined) {
      // Vanguard shield
      g.poly([0, -7, 6, -3, 4, 5, 0, 8, -4, 5, -6, -3]);
      g.fill({ color: 0xffffff, alpha: 0.9 });
    } else if (unit.cooldowns?.longshot !== undefined || unit.attack_range >= 3 || unit.max_hp === 80) {
      // Sniper crosshair
      g.circle(0, 0, 6);
      g.stroke({ color: 0xffffff, width: 1.2 });
      g.moveTo(-8, 0); g.lineTo(8, 0); g.stroke({ color: 0xffffff, width: 1.2 });
      g.moveTo(0, -8); g.lineTo(0, 8); g.stroke({ color: 0xffffff, width: 1.2 });
    } else if (unit.cooldowns?.fury !== undefined || (unit.max_hp === 120 && unit.attack_range === 1)) {
      // Berserker rage blades
      g.poly([-5, -6, -3, -7, 5, 6, 3, 7]); g.fill({ color: 0xffffff });
      g.poly([5, -6, 3, -7, -5, 6, -3, 7]); g.fill({ color: 0xffffff });
    } else if (unit.attack_range >= 2 || unit.cooldowns?.bolt !== undefined) {
      // Ranger bow/crosshair
      g.circle(0, 0, 6);
      g.stroke({ color: 0xffffff, width: 1.5 });
      g.circle(0, 0, 1.8);
      g.fill({ color: 0xffffff });
    } else if (unit.max_energy === 6 || unit.cooldowns?.mend !== undefined) {
      // Warden medic cross
      g.rect(-2, -6, 4, 12);
      g.rect(-6, -2, 12, 4);
      g.fill({ color: 0xffffff, alpha: 0.9 });
    }

    // Ghost label
    const label = new PIXI.Text({
      text: 'GHOST',
      style: {
        fontSize: 8,
        fill: 0xffffff,
        fontFamily: 'JetBrains Mono, monospace',
        fontWeight: 'bold',
      },
    });
    label.anchor.set(0.5);
    label.y = HEX_SIZE * 0.75;
    ghostContainer.addChild(g);
    ghostContainer.addChild(label);
    this.ghostLayer.addChild(ghostContainer);

    // Directional dashed spline connecting start to ghost destination
    const spline = new PIXI.Graphics();
    spline.moveTo(startPixel.x, startPixel.y);
    spline.lineTo(destPixel.x, destPixel.y);
    spline.stroke({ color: glowColor, width: 2, alpha: 0.8 });

    // Arrow pointer at destination
    const angle = Math.atan2(destPixel.y - startPixel.y, destPixel.x - startPixel.x);
    const arrowLen = 8;
    const arrowX = destPixel.x - Math.cos(angle) * (HEX_SIZE * 0.65);
    const arrowY = destPixel.y - Math.sin(angle) * (HEX_SIZE * 0.65);
    spline.moveTo(arrowX, arrowY);
    spline.lineTo(
      arrowX - arrowLen * Math.cos(angle - Math.PI / 6),
      arrowY - arrowLen * Math.sin(angle - Math.PI / 6)
    );
    spline.moveTo(arrowX, arrowY);
    spline.lineTo(
      arrowX - arrowLen * Math.cos(angle + Math.PI / 6),
      arrowY - arrowLen * Math.sin(angle + Math.PI / 6)
    );
    spline.stroke({ color: 0xffffff, width: 2, alpha: 0.9 });
    this.ghostLayer.addChild(spline);
  }

  clearGhost(): void {
    this.ghostLayer.removeChildren();
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

  clearOverlays(): void {
    this.overlayLayer.removeChildren();
    this.overlayLayer.addChild(this.selectionRing);
    this.overlayLayer.addChild(this.raycastLine);
    this.selectionRing.clear();
    this.raycastLine.clear();
    this.pathLayer.removeChildren();
    this.clearGhost();
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

  getWorldContainer(): PIXI.Container {
    return this.worldContainer;
  }

  getFxLayer(): PIXI.Container {
    return this.fxLayer;
  }

  getApp(): PIXI.Application {
    return this.app;
  }
}
