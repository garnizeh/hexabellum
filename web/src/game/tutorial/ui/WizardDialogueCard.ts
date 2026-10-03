import { TutorialSound } from '../TutorialSound';

export class WizardDialogueCard {
  private container: HTMLElement;
  private card: HTMLElement;
  private avatarEl: HTMLElement;
  private nameEl: HTMLElement;
  private textEl: HTMLElement;
  private btnGroup: HTMLElement;
  private btnContinue: HTMLButtonElement;
  private btnSkip: HTMLButtonElement;
  private btnAutoDemo: HTMLButtonElement;

  private currentLines: string[] = [];
  private currentLineIdx: number = 0;
  private typewriterInterval: number | null = null;
  private onContinueCb: (() => void) | null = null;
  private onSkipCb: (() => void) | null = null;
  private onAutoDemoCb: (() => void) | null = null;
  private isTyping: boolean = false;
  private fullTextOfCurrentLine: string = '';

  constructor() {
    this.container = document.createElement('div');
    this.container.id = 'wizard-dialogue-container';
    this.container.className = 'wizard-container';
    this.container.style.display = 'none';

    this.card = document.createElement('div');
    this.card.className = 'wizard-card glass-panel';
    this.card.setAttribute('role', 'status');
    this.card.setAttribute('aria-live', 'polite');

    const leftCol = document.createElement('div');
    leftCol.className = 'wizard-avatar-col';

    this.avatarEl = document.createElement('div');
    this.avatarEl.className = 'wizard-avatar';
    this.avatarEl.innerHTML = `
      <div class="archmage-glow"></div>
      <div class="archmage-sigil">🧙</div>
    `;

    this.nameEl = document.createElement('div');
    this.nameEl.className = 'wizard-name';
    this.nameEl.textContent = 'Archmage of Convergence';

    leftCol.appendChild(this.avatarEl);
    leftCol.appendChild(this.nameEl);

    const rightCol = document.createElement('div');
    rightCol.className = 'wizard-body-col';

    this.textEl = document.createElement('div');
    this.textEl.className = 'wizard-text';

    this.btnGroup = document.createElement('div');
    this.btnGroup.className = 'wizard-actions';

    this.btnContinue = document.createElement('button');
    this.btnContinue.className = 'btn-wizard btn-wizard-primary';
    this.btnContinue.innerHTML = '<span>Continue</span> <kbd>[Space]</kbd>';
    this.btnContinue.addEventListener('click', () => this.handleContinue());

    this.btnSkip = document.createElement('button');
    this.btnSkip.className = 'btn-wizard btn-wizard-ghost';
    this.btnSkip.textContent = 'Skip';
    this.btnSkip.addEventListener('click', () => {
      this.stopTypewriter();
      this.hide();
      if (this.onSkipCb) this.onSkipCb();
    });

    this.btnAutoDemo = document.createElement('button');
    this.btnAutoDemo.className = 'btn-wizard btn-wizard-demo';
    this.btnAutoDemo.style.display = 'none';
    this.btnAutoDemo.textContent = '👁️ Watch me show you';
    this.btnAutoDemo.addEventListener('click', () => {
      if (this.onAutoDemoCb) this.onAutoDemoCb();
    });

    this.btnGroup.appendChild(this.btnAutoDemo);
    this.btnGroup.appendChild(this.btnSkip);
    this.btnGroup.appendChild(this.btnContinue);

    rightCol.appendChild(this.textEl);
    rightCol.appendChild(this.btnGroup);

    this.card.appendChild(leftCol);
    this.card.appendChild(rightCol);
    this.container.appendChild(this.card);
    document.body.appendChild(this.container);

    // Keyboard listener for Space/Enter
    window.addEventListener('keydown', (e) => {
      if (this.container.style.display !== 'none' && (e.code === 'Space' || e.code === 'Enter')) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        e.preventDefault();
        this.handleContinue();
      }
    });
  }

  public showDialogue(
    lines: string[],
    emotion: string = 'Neutral',
    onContinue: () => void,
    onSkip?: () => void
  ) {
    this.currentLines = lines;
    this.currentLineIdx = 0;
    this.onContinueCb = onContinue;
    this.onSkipCb = onSkip || null;
    this.btnAutoDemo.style.display = 'none';
    this.container.style.display = 'flex';
    this.card.classList.remove('card-soft-fail');

    this.setEmotion(emotion);
    this.renderCurrentLine();
  }

  public showSoftFail(message: string) {
    this.container.style.display = 'flex';
    this.card.classList.add('card-soft-fail');
    TutorialSound.playSoftFail();
    this.setEmotion('Stern');
    this.currentLines = [message];
    this.currentLineIdx = 0;
    this.renderCurrentLine(true);
  }

  public showAutoDemoButton(onDemo: () => void) {
    this.onAutoDemoCb = onDemo;
    this.btnAutoDemo.style.display = 'inline-flex';
  }

  public hideAutoDemoButton() {
    this.btnAutoDemo.style.display = 'none';
    this.onAutoDemoCb = null;
  }

  public hide() {
    this.stopTypewriter();
    this.container.style.display = 'none';
  }

  public isVisible(): boolean {
    return this.container.style.display !== 'none';
  }

  private handleContinue() {
    this.stopTypewriter();
    if (this.currentLineIdx < this.currentLines.length - 1) {
      this.currentLineIdx++;
      this.renderCurrentLine();
    } else {
      this.hide();
      if (this.onContinueCb) {
        this.onContinueCb();
      }
    }
  }

  private setEmotion(emotion: string) {
    this.avatarEl.setAttribute('data-emotion', emotion);
    if (emotion === 'Enlightened') {
      this.nameEl.style.color = '#00e676';
    } else if (emotion === 'Pleased') {
      this.nameEl.style.color = '#00d2ff';
    } else if (emotion === 'Stern') {
      this.nameEl.style.color = '#ffb300';
    } else {
      this.nameEl.style.color = '#cbd5e1';
    }
  }

  private renderCurrentLine(isWarning = false) {
    this.stopTypewriter();
    const line = this.currentLines[this.currentLineIdx] || '';
    this.fullTextOfCurrentLine = line;
    this.textEl.textContent = '';
    this.isTyping = true;

    if (!isWarning) {
      TutorialSound.playArchmageChime();
    }

    let charIdx = 0;
    this.typewriterInterval = window.setInterval(() => {
      if (charIdx < line.length) {
        this.textEl.textContent += line[charIdx];
        charIdx++;
      } else {
        this.stopTypewriter();
        this.isTyping = false;
      }
    }, 18);
  }

  private stopTypewriter() {
    if (this.typewriterInterval !== null) {
      clearInterval(this.typewriterInterval);
      this.typewriterInterval = null;
    }
    this.isTyping = false;
  }
}
