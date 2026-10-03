export class SynchronizedTurnTimer {
  private deadlineUnixMs: number | null = null;
  private totalDurationMs: number = 30000;
  private intervalId: number | null = null;
  private onTickCallback: ((remainingSecs: number, phaseRatio: number) => void) | null = null;
  private onExpireCallback: (() => void) | null = null;

  start(
    deadlineUnixMs: number,
    onTick: (s: number, r: number) => void,
    onExpire: () => void,
    durationMs: number = 30000
  ): void {
    this.stop();
    this.deadlineUnixMs = deadlineUnixMs;
    this.totalDurationMs = durationMs > 0 ? durationMs : 30000;
    this.onTickCallback = onTick;
    this.onExpireCallback = onExpire;

    this.update();
    this.intervalId = window.setInterval(() => this.update(), 100);
  }

  stop(): void {
    if (this.intervalId !== null) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
    this.deadlineUnixMs = null;
  }

  private update(): void {
    if (!this.deadlineUnixMs) return;

    const remainingMs = Math.max(0, this.deadlineUnixMs - Date.now());
    const remainingSecs = Math.ceil(remainingMs / 1000);
    const ratio = Math.max(0, Math.min(1, remainingMs / this.totalDurationMs));

    this.onTickCallback?.(remainingSecs, ratio);

    if (remainingMs <= 0) {
      this.stop();
      this.onExpireCallback?.();
    }
  }
}

export class TurnTimer {
  private duration: number;
  private remaining: number;
  private deadlineUnixMs: number | null = null;
  private intervalId: number | null = null;
  private onExpire: (() => void) | null = null;
  private displayElement: HTMLElement | null = null;

  constructor(durationSeconds: number = 30) {
    this.duration = durationSeconds;
    this.remaining = durationSeconds;
  }

  setDisplay(element: HTMLElement): void {
    this.displayElement = element;
    this.updateDisplay();
  }

  start(onExpire: () => void): void {
    this.stop();
    this.deadlineUnixMs = null;
    this.remaining = this.duration;
    this.onExpire = onExpire;
    this.updateDisplay();

    this.intervalId = window.setInterval(() => {
      this.remaining--;
      this.updateDisplay();

      if (this.remaining <= 0) {
        this.stop();
        this.onExpire?.();
      }
    }, 1000);
  }

  startWithDeadline(deadlineUnixMs: number, onExpire?: () => void): void {
    this.stop();
    this.deadlineUnixMs = deadlineUnixMs;
    if (onExpire) this.onExpire = onExpire;

    this.updateDeadlineTick();
    this.intervalId = window.setInterval(() => this.updateDeadlineTick(), 100);
  }

  stop(): void {
    if (this.intervalId !== null) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
    this.deadlineUnixMs = null;
  }

  private updateDeadlineTick(): void {
    if (!this.deadlineUnixMs) return;

    const remainingMs = Math.max(0, this.deadlineUnixMs - Date.now());
    this.remaining = Math.ceil(remainingMs / 1000);
    this.updateDisplay();

    if (remainingMs <= 0) {
      this.stop();
      this.onExpire?.();
    }
  }

  private updateDisplay(): void {
    if (!this.displayElement) return;

    const minutes = Math.floor(this.remaining / 60);
    const seconds = this.remaining % 60;
    this.displayElement.textContent = `${minutes}:${seconds.toString().padStart(2, '0')}`;

    // Dynamic warning styling
    if (this.remaining <= 5) {
      this.displayElement.style.color = '#ff1744';
      this.displayElement.style.textShadow = '0 0 10px rgba(255, 23, 68, 0.8)';
    } else if (this.remaining <= 10) {
      this.displayElement.style.color = '#ffea00';
      this.displayElement.style.textShadow = '0 0 8px rgba(255, 234, 0, 0.6)';
    } else {
      this.displayElement.style.color = '#00d2ff';
      this.displayElement.style.textShadow = 'none';
    }
  }
}
