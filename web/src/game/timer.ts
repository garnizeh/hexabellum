export class TurnTimer {
  private duration: number;
  private remaining: number;
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

  stop(): void {
    if (this.intervalId !== null) {
      clearInterval(this.intervalId);
      this.intervalId = null;
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
