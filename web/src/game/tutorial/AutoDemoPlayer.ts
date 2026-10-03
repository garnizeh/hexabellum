export class AutoDemoPlayer {
  private ghostCursor: HTMLElement;
  private isRunning: boolean = false;

  constructor() {
    this.ghostCursor = document.createElement('div');
    this.ghostCursor.id = 'tutorial-ghost-cursor';
    this.ghostCursor.className = 'ghost-cursor';
    this.ghostCursor.style.display = 'none';
    this.ghostCursor.innerHTML = `
      <div class="cursor-pointer">👆</div>
      <div class="cursor-ripple"></div>
    `;
    document.body.appendChild(this.ghostCursor);
  }

  public playDemo(
    fromPixel: { x: number; y: number },
    toPixel: { x: number; y: number },
    onComplete: () => void
  ) {
    if (this.isRunning) return;
    this.isRunning = true;
    this.ghostCursor.style.display = 'block';
    this.ghostCursor.style.left = `${fromPixel.x}px`;
    this.ghostCursor.style.top = `${fromPixel.y}px`;
    this.ghostCursor.style.transition = 'none';

    // 1. Initial click at start
    setTimeout(() => {
      this.ghostCursor.classList.add('clicking');

      setTimeout(() => {
        this.ghostCursor.classList.remove('clicking');
        // 2. Smoothly glide to target hex
        this.ghostCursor.style.transition = 'top 1.4s cubic-bezier(0.25, 1, 0.5, 1), left 1.4s cubic-bezier(0.25, 1, 0.5, 1)';
        this.ghostCursor.style.left = `${toPixel.x}px`;
        this.ghostCursor.style.top = `${toPixel.y}px`;

        // 3. Click destination
        setTimeout(() => {
          this.ghostCursor.classList.add('clicking');

          setTimeout(() => {
            this.ghostCursor.style.display = 'none';
            this.ghostCursor.classList.remove('clicking');
            this.isRunning = false;
            onComplete();
          }, 600);
        }, 1500);
      }, 400);
    }, 100);
  }

  public stop() {
    this.isRunning = false;
    this.ghostCursor.style.display = 'none';
  }
}
