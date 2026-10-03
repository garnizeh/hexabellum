import * as PIXI from 'pixi.js';

export class CameraController {
  private container: PIXI.Container;
  private canvas: HTMLCanvasElement;
  private isDragging = false;
  private dragStartX = 0;
  private dragStartY = 0;
  private containerStartX = 0;
  private containerStartY = 0;
  private draggedDistance = 0;
  private lastDragEndTime = 0;

  public zoom = 1.0;
  public readonly minZoom = 0.5;
  public readonly maxZoom = 2.0;

  // Keyboard pan state
  private activeKeys = new Set<string>();
  private keyPanSpeed = 14;
  private animFrameId: number | null = null;

  // Target centering interpolation
  private targetX: number | null = null;
  private targetY: number | null = null;

  // Callback to retrieve player's controlled hero position for [Space]
  private getControlledHeroPos: (() => { x: number; y: number } | null) | null = null;

  constructor(container: PIXI.Container, canvas: HTMLCanvasElement) {
    this.container = container;
    this.canvas = canvas;

    this.setupPointerInteractions();
    this.setupWheelInteraction();
    this.setupKeyboardPan();
    this.startUpdateLoop();
  }

  public setHeroPositionProvider(provider: () => { x: number; y: number } | null): void {
    this.getControlledHeroPos = provider;
  }

  public hasDraggedRecently(): boolean {
    return Date.now() - this.lastDragEndTime < 150 || this.draggedDistance > 6;
  }

  public getZoom(): number {
    return this.zoom;
  }

  public setZoom(zoom: number, screenWidth?: number, screenHeight?: number): void {
    const sw = screenWidth ?? this.canvas.width;
    const sh = screenHeight ?? this.canvas.height;
    const cx = sw / 2;
    const cy = sh / 2;
    const worldPos = {
      x: (cx - this.container.x) / this.zoom,
      y: (cy - this.container.y) / this.zoom,
    };

    this.zoom = Math.min(this.maxZoom, Math.max(this.minZoom, zoom));
    this.container.scale.set(this.zoom);
    this.container.position.set(
      cx - worldPos.x * this.zoom,
      cy - worldPos.y * this.zoom
    );
    this.clampToBounds(sw, sh);
  }

  public centerOn(
    worldX: number,
    worldY: number,
    screenWidth: number,
    screenHeight: number,
    smooth: boolean = true
  ): void {
    const destX = screenWidth / 2 - worldX * this.zoom;
    const destY = screenHeight / 2 - worldY * this.zoom;

    if (smooth) {
      this.targetX = destX;
      this.targetY = destY;
    } else {
      this.container.position.set(destX, destY);
      this.clampToBounds(screenWidth, screenHeight);
      this.targetX = null;
      this.targetY = null;
    }
  }

  public centerOnHero(screenWidth?: number, screenHeight?: number): boolean {
    if (!this.getControlledHeroPos) return false;
    const pos = this.getControlledHeroPos();
    if (!pos) return false;

    const sw = screenWidth ?? this.canvas.clientWidth ?? window.innerWidth;
    const sh = screenHeight ?? this.canvas.clientHeight ?? window.innerHeight;
    this.centerOn(pos.x, pos.y, sw, sh, true);
    return true;
  }

  private setupPointerInteractions(): void {
    this.canvas.addEventListener('pointerdown', (e: PointerEvent) => {
      // Allow dragging with left mouse (button 0) or middle mouse (button 1)
      if (e.button !== 0 && e.button !== 1) return;

      this.isDragging = true;
      this.dragStartX = e.clientX;
      this.dragStartY = e.clientY;
      this.containerStartX = this.container.x;
      this.containerStartY = this.container.y;
      this.draggedDistance = 0;
      this.targetX = null;
      this.targetY = null;
    });

    window.addEventListener('pointermove', (e: PointerEvent) => {
      if (!this.isDragging) return;

      const dx = e.clientX - this.dragStartX;
      const dy = e.clientY - this.dragStartY;
      this.draggedDistance = Math.hypot(dx, dy);

      this.container.position.set(
        this.containerStartX + dx,
        this.containerStartY + dy
      );
      this.clampToBounds(this.canvas.clientWidth, this.canvas.clientHeight);
    });

    const stopDrag = () => {
      if (this.isDragging) {
        this.isDragging = false;
        if (this.draggedDistance > 5) {
          this.lastDragEndTime = Date.now();
        }
      }
    };

    window.addEventListener('pointerup', stopDrag);
    window.addEventListener('pointercancel', stopDrag);
  }

  private setupWheelInteraction(): void {
    this.canvas.addEventListener(
      'wheel',
      (e: WheelEvent) => {
        e.preventDefault();
        const sw = this.canvas.clientWidth || window.innerWidth;
        const sh = this.canvas.clientHeight || window.innerHeight;

        const zoomFactor = e.deltaY < 0 ? 1.12 : 0.89;
        const newZoom = Math.min(
          this.maxZoom,
          Math.max(this.minZoom, this.zoom * zoomFactor)
        );

        // Zoom centered on cursor
        const mouseX = e.clientX;
        const mouseY = e.clientY;
        const worldPos = {
          x: (mouseX - this.container.x) / this.zoom,
          y: (mouseY - this.container.y) / this.zoom,
        };

        this.zoom = newZoom;
        this.container.scale.set(this.zoom);
        this.container.position.set(
          mouseX - worldPos.x * this.zoom,
          mouseY - worldPos.y * this.zoom
        );
        this.clampToBounds(sw, sh);
        this.targetX = null;
        this.targetY = null;
      },
      { passive: false }
    );
  }

  private setupKeyboardPan(): void {
    window.addEventListener('keydown', (e: KeyboardEvent) => {
      const targetTag = (e.target as HTMLElement)?.tagName?.toLowerCase();
      if (targetTag === 'input' || targetTag === 'textarea') return;

      // Spacebar: Center camera on player's controlled hero
      if (e.code === 'Space') {
        const handled = this.centerOnHero();
        if (handled) {
          e.preventDefault();
          return;
        }
      }

      const key = e.key.toLowerCase();
      if (
        key === 'w' ||
        key === 'a' ||
        key === 's' ||
        key === 'd' ||
        key === 'arrowup' ||
        key === 'arrowdown' ||
        key === 'arrowleft' ||
        key === 'arrowright'
      ) {
        this.activeKeys.add(key);
        this.targetX = null;
        this.targetY = null;
      }
    });

    window.addEventListener('keyup', (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      this.activeKeys.delete(key);
    });
  }

  private startUpdateLoop(): void {
    const loop = () => {
      const sw = this.canvas.clientWidth || window.innerWidth;
      const sh = this.canvas.clientHeight || window.innerHeight;

      // Smooth interpolation towards target if centering
      if (this.targetX !== null && this.targetY !== null) {
        const dx = this.targetX - this.container.x;
        const dy = this.targetY - this.container.y;
        if (Math.abs(dx) < 1 && Math.abs(dy) < 1) {
          this.container.position.set(this.targetX, this.targetY);
          this.targetX = null;
          this.targetY = null;
        } else {
          this.container.position.set(
            this.container.x + dx * 0.18,
            this.container.y + dy * 0.18
          );
        }
        this.clampToBounds(sw, sh);
      }

      // Handle keyboard panning
      if (this.activeKeys.size > 0 && !this.isDragging) {
        let dx = 0;
        let dy = 0;
        if (this.activeKeys.has('w') || this.activeKeys.has('arrowup')) dy += this.keyPanSpeed;
        if (this.activeKeys.has('s') || this.activeKeys.has('arrowdown')) dy -= this.keyPanSpeed;
        if (this.activeKeys.has('a') || this.activeKeys.has('arrowleft')) dx += this.keyPanSpeed;
        if (this.activeKeys.has('d') || this.activeKeys.has('arrowright')) dx -= this.keyPanSpeed;

        if (dx !== 0 || dy !== 0) {
          this.container.position.set(this.container.x + dx, this.container.y + dy);
          this.clampToBounds(sw, sh);
        }
      }

      this.animFrameId = requestAnimationFrame(loop);
    };

    this.animFrameId = requestAnimationFrame(loop);
  }

  public clampToBounds(screenWidth: number, screenHeight: number): void {
    // Radius 8 arena boundary: map radius in world units is ~420px.
    // Clamping allows panning while keeping map visible on screen.
    const mapRadiusWorld = 520 * this.zoom;
    const minX = screenWidth / 2 - mapRadiusWorld;
    const maxX = screenWidth / 2 + mapRadiusWorld;
    const minY = screenHeight / 2 - mapRadiusWorld;
    const maxY = screenHeight / 2 + mapRadiusWorld;

    this.container.x = Math.max(minX, Math.min(maxX, this.container.x));
    this.container.y = Math.max(minY, Math.min(maxY, this.container.y));
  }

  public destroy(): void {
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
    }
  }
}
