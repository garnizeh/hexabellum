import { HexRenderer, HEX_SIZE } from '../game/renderer';
import { SnapshotDto, UnitDto } from '../game/types';

export type PingType = 'danger' | 'assist' | 'sight' | 'on_my_way';

export interface PingEvent {
  type: PingType;
  worldX: number;
  worldY: number;
  label: string;
  color: string;
  timestamp: number;
}

export class TacticalMinimap {
  private containerEl: HTMLElement | null = null;
  private canvasEl: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private renderer: HexRenderer | null = null;
  private currentSnapshot: SnapshotDto | null = null;
  private pings: PingEvent[] = [];
  private activePingType: PingType | null = null;

  private readonly radarWidth = 190;
  private readonly radarHeight = 190;
  private readonly radarScale = 0.22; // Maps world coordinates to radar canvas

  private onPingCreatedCb: ((ping: PingEvent) => void) | null = null;

  constructor() {
    this.createDom();
    this.setupInteractions();
  }

  setRenderer(renderer: HexRenderer): void {
    this.renderer = renderer;
  }

  setOnPing(cb: (ping: PingEvent) => void): void {
    this.onPingCreatedCb = cb;
  }

  private createDom(): void {
    if (document.getElementById('hb-tactical-minimap')) {
      this.containerEl = document.getElementById('hb-tactical-minimap');
      this.canvasEl = this.containerEl?.querySelector('canvas') ?? null;
      if (this.canvasEl) this.ctx = this.canvasEl.getContext('2d');
      return;
    }

    const panel = document.createElement('div');
    panel.id = 'hb-tactical-minimap';
    panel.className = 'hb-minimap-panel';
    panel.innerHTML = `
      <div class="hb-minimap-header">
        <div class="hb-minimap-title">
          <span style="color:#38bdf8;">🌐 RADAR</span>
          <span style="font-size:10px; color:#94a3b8; font-family:'JetBrains Mono',monospace;">R=8</span>
        </div>
        <button id="btn-minimap-center" class="hb-minimap-tool-btn" title="[C] Center Camera on Hero">
          <span>⊙ C</span>
        </button>
      </div>

      <div class="hb-minimap-canvas-wrap">
        <canvas id="hb-minimap-canvas" width="${this.radarWidth}" height="${this.radarHeight}"></canvas>
      </div>

      <div class="hb-minimap-ping-bar">
        <button class="hb-ping-btn danger" data-ping="danger" title="Danger / Fall Back">⚠️</button>
        <button class="hb-ping-btn assist" data-ping="assist" title="Assist / Attack Here">🚩</button>
        <button class="hb-ping-btn sight" data-ping="sight" title="Need Vision / Enemy Missing">👁️</button>
        <button class="hb-ping-btn omw" data-ping="on_my_way" title="On My Way">🏹</button>
      </div>
    `;

    document.body.appendChild(panel);
    this.containerEl = panel;
    this.canvasEl = panel.querySelector('#hb-minimap-canvas') as HTMLCanvasElement;
    if (this.canvasEl) this.ctx = this.canvasEl.getContext('2d');
  }

  private setupInteractions(): void {
    const centerBtn = this.containerEl?.querySelector('#btn-minimap-center');
    centerBtn?.addEventListener('click', () => {
      this.renderer?.getCamera().centerOnHero();
    });

    const pingButtons = this.containerEl?.querySelectorAll<HTMLButtonElement>('.hb-ping-btn');
    pingButtons?.forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const pType = btn.getAttribute('data-ping') as PingType;
        if (this.activePingType === pType) {
          this.activePingType = null;
          btn.classList.remove('active');
        } else {
          pingButtons.forEach((b) => b.classList.remove('active'));
          this.activePingType = pType;
          btn.classList.add('active');
        }
      });
    });

    // Radar canvas click / drag to center camera or drop ping
    if (this.canvasEl) {
      let isMouseDown = false;

      const handleMinimapPointer = (e: MouseEvent) => {
        if (!this.canvasEl || !this.renderer) return;
        const rect = this.canvasEl.getBoundingClientRect();
        const clickX = e.clientX - rect.left;
        const clickY = e.clientY - rect.top;

        // Convert radar pixels back to world coordinates
        const centerX = this.radarWidth / 2;
        const centerY = this.radarHeight / 2;
        const worldX = (clickX - centerX) / this.radarScale;
        const worldY = (clickY - centerY) / this.radarScale;

        if (this.activePingType) {
          // Drop a ping at this location
          this.triggerPing(this.activePingType, worldX, worldY);
          this.activePingType = null;
          pingButtons?.forEach((b) => b.classList.remove('active'));
          return;
        }

        // Center camera at world position
        const app = this.renderer.getApp();
        const sw = app.screen.width;
        const sh = app.screen.height;
        this.renderer.getCamera().centerOn(worldX, worldY, sw, sh, true);
      };

      this.canvasEl.addEventListener('mousedown', (e) => {
        isMouseDown = true;
        handleMinimapPointer(e);
      });

      window.addEventListener('mousemove', (e) => {
        if (isMouseDown && !this.activePingType) {
          handleMinimapPointer(e);
        }
      });

      window.addEventListener('mouseup', () => {
        isMouseDown = false;
      });
    }
  }

  public triggerPing(type: PingType, worldX: number, worldY: number): void {
    let label = 'PING';
    let color = '#38bdf8';
    switch (type) {
      case 'danger':
        label = '⚠️ DANGER / FALL BACK';
        color = '#ef4444';
        break;
      case 'assist':
        label = '🚩 ASSIST / ATTACK';
        color = '#f59e0b';
        break;
      case 'sight':
        label = '👁️ NEED VISION';
        color = '#eab308';
        break;
      case 'on_my_way':
        label = '🏹 ON MY WAY';
        color = '#10b981';
        break;
    }

    const event: PingEvent = {
      type,
      worldX,
      worldY,
      label,
      color,
      timestamp: Date.now(),
    };
    this.pings.push(event);
    this.onPingCreatedCb?.(event);
    this.render();
  }

  public update(snapshot: SnapshotDto): void {
    this.currentSnapshot = snapshot;
    this.render();
  }

  public render(): void {
    if (!this.canvasEl || !this.ctx || !this.renderer) return;

    const ctx = this.ctx;
    const cw = this.radarWidth;
    const ch = this.radarHeight;
    const cx = cw / 2;
    const cy = ch / 2;

    ctx.clearRect(0, 0, cw, ch);

    // 1. Radar background
    ctx.fillStyle = '#070a13';
    ctx.fillRect(0, 0, cw, ch);

    // Arena boundary hex / circle
    const arenaRadius = HEX_SIZE * 8 * 1.75 * this.radarScale;
    ctx.beginPath();
    ctx.arc(cx, cy, arenaRadius, 0, Math.PI * 2);
    ctx.fillStyle = '#0d1322';
    ctx.fill();
    ctx.strokeStyle = 'rgba(56, 189, 248, 0.25)';
    ctx.lineWidth = 1.5;
    ctx.stroke();

    // Subtle coordinate crosshair
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.06)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cx, 10); ctx.lineTo(cx, ch - 10);
    ctx.moveTo(10, cy); ctx.lineTo(cw - 10, cy);
    ctx.stroke();

    // 2. Base Zones
    const playerTeam = this.currentSnapshot?.player_team ?? 0;
    const baseZones = this.currentSnapshot?.base_zones ?? [
      { team: 0, center: { q: -7, r: 0 }, radius: 2 },
      { team: 1, center: { q: 7, r: 0 }, radius: 2 },
    ];

    for (const bz of baseZones) {
      const bzWorld = this.renderer.hexToPixel(bz.center.q, bz.center.r);
      const bx = cx + bzWorld.x * this.radarScale;
      const by = cy + bzWorld.y * this.radarScale;
      const br = HEX_SIZE * 2.2 * this.radarScale;

      ctx.beginPath();
      ctx.arc(bx, by, br, 0, Math.PI * 2);
      ctx.fillStyle = bz.team === 0 ? 'rgba(56, 189, 248, 0.18)' : 'rgba(244, 63, 94, 0.18)';
      ctx.fill();
      ctx.strokeStyle = bz.team === 0 ? '#38bdf8' : '#f43f5e';
      ctx.lineWidth = 1;
      ctx.stroke();
    }

    // 3. Central Ancient Vault
    const vaultWorld = this.renderer.hexToPixel(0, 0);
    const vx = cx + vaultWorld.x * this.radarScale;
    const vy = cy + vaultWorld.y * this.radarScale;
    ctx.fillStyle = '#f59e0b';
    ctx.beginPath();
    ctx.arc(vx, vy, 4, 0, Math.PI * 2);
    ctx.fill();

    // 4. Units & Structures
    if (this.currentSnapshot && this.currentSnapshot.units) {
      for (const unit of this.currentSnapshot.units) {
        if (unit.hp <= 0 || unit.life_state === 'dead_awaiting_respawn') continue;
        const uWorld = this.renderer.hexToPixel(unit.pos.q, unit.pos.r);
        const ux = cx + uWorld.x * this.radarScale;
        const uy = cy + uWorld.y * this.radarScale;

        const isFriendly = unit.team === playerTeam;
        const isCore = unit.kind === 'Core';
        const isTower = unit.kind === 'Tower';
        const isSpawner = unit.kind === 'Spawner' || unit.kind === 'SpawnerTower';
        const isHero = unit.kind === 'Hero';
        const isMinion = unit.kind === 'Minion';
        const isNeutral = unit.team === 255 || unit.kind === 'Neutral' || unit.kind === 'NeutralGuardian';

        if (isCore) {
          // Sovereign Core (diamond)
          ctx.fillStyle = unit.team === 0 ? '#38bdf8' : '#f43f5e';
          ctx.beginPath();
          ctx.moveTo(ux, uy - 6);
          ctx.lineTo(ux + 6, uy);
          ctx.lineTo(ux, uy + 6);
          ctx.lineTo(ux - 6, uy);
          ctx.closePath();
          ctx.fill();
          ctx.strokeStyle = '#ffffff';
          ctx.lineWidth = 1;
          ctx.stroke();
        } else if (isTower) {
          // Defensive Tower (square)
          ctx.fillStyle = isFriendly ? '#38bdf8' : '#f43f5e';
          ctx.fillRect(ux - 3.5, uy - 3.5, 7, 7);
          ctx.strokeStyle = '#ffffff';
          ctx.lineWidth = 0.8;
          ctx.strokeRect(ux - 3.5, uy - 3.5, 7, 7);
        } else if (isSpawner) {
          ctx.fillStyle = isFriendly ? '#818cf8' : '#fb923c';
          ctx.beginPath();
          ctx.arc(ux, uy, 3.5, 0, Math.PI * 2);
          ctx.fill();
        } else if (isHero) {
          // Hero Blip: Solid dot with white stroke
          ctx.beginPath();
          ctx.arc(ux, uy, 5, 0, Math.PI * 2);
          ctx.fillStyle = isFriendly ? '#2563eb' : '#dc2626';
          ctx.fill();
          ctx.strokeStyle = '#ffffff';
          ctx.lineWidth = 1.2;
          ctx.stroke();

          // Initial letter
          const initial = unit.hero_id ? unit.hero_id[0].toUpperCase() : 'H';
          ctx.fillStyle = '#ffffff';
          ctx.font = 'bold 6.5px JetBrains Mono, monospace';
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillText(initial, ux, uy + 0.5);
        } else if (isMinion) {
          ctx.beginPath();
          ctx.arc(ux, uy, 2, 0, Math.PI * 2);
          ctx.fillStyle = isFriendly ? '#93c5fd' : '#fca5a5';
          ctx.fill();
        } else if (isNeutral) {
          ctx.fillStyle = '#d97706';
          ctx.beginPath();
          ctx.arc(ux, uy, 3, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    }

    // 5. Camera Frustum Viewport Rectangle
    const cam = this.renderer.getCamera();
    const app = this.renderer.getApp();
    const sw = app.screen.width;
    const sh = app.screen.height;
    const worldContainer = this.renderer.getWorldContainer();

    const worldLeft = -worldContainer.x / cam.zoom;
    const worldTop = -worldContainer.y / cam.zoom;
    const worldW = sw / cam.zoom;
    const worldH = sh / cam.zoom;

    const frustumX = cx + worldLeft * this.radarScale;
    const frustumY = cy + worldTop * this.radarScale;
    const frustumW = worldW * this.radarScale;
    const frustumH = worldH * this.radarScale;

    ctx.strokeStyle = '#38bdf8';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(frustumX, frustumY, frustumW, frustumH);
    ctx.fillStyle = 'rgba(56, 189, 248, 0.1)';
    ctx.fillRect(frustumX, frustumY, frustumW, frustumH);

    // 6. Active Pings (Fade over 3 seconds)
    const now = Date.now();
    this.pings = this.pings.filter((p) => now - p.timestamp < 3000);

    for (const p of this.pings) {
      const px = cx + p.worldX * this.radarScale;
      const py = cy + p.worldY * this.radarScale;
      const elapsed = now - p.timestamp;
      const alpha = Math.max(0, 1 - elapsed / 3000);
      const radius = 5 + (elapsed / 3000) * 15;

      ctx.save();
      ctx.globalAlpha = alpha;
      ctx.strokeStyle = p.color;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(px, py, radius, 0, Math.PI * 2);
      ctx.stroke();

      ctx.fillStyle = p.color;
      ctx.beginPath();
      ctx.arc(px, py, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }
  }
}
