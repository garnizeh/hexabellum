import { CalloutConfig } from '../ScenarioLoader';

export class CalloutOverlay {
  private container: HTMLElement;
  private pointerCard: HTMLElement;
  private titleEl: HTMLElement;
  private descEl: HTMLElement;
  private arrowEl: HTMLElement;

  constructor() {
    this.container = document.createElement('div');
    this.container.id = 'callout-overlay';
    this.container.className = 'callout-overlay';
    this.container.style.display = 'none';

    this.pointerCard = document.createElement('div');
    this.pointerCard.className = 'callout-card glass-panel';

    this.arrowEl = document.createElement('div');
    this.arrowEl.className = 'callout-arrow';

    this.titleEl = document.createElement('h4');
    this.titleEl.className = 'callout-title';

    this.descEl = document.createElement('p');
    this.descEl.className = 'callout-desc';

    this.pointerCard.appendChild(this.arrowEl);
    this.pointerCard.appendChild(this.titleEl);
    this.pointerCard.appendChild(this.descEl);
    this.container.appendChild(this.pointerCard);

    document.body.appendChild(this.container);
  }

  public showCallout(config: CalloutConfig) {
    const target = document.querySelector(config.target_selector);
    if (!target) {
      console.warn(`Callout target not found: ${config.target_selector}`);
      return;
    }

    this.titleEl.textContent = config.title;
    this.descEl.textContent = config.description;
    this.container.style.display = 'block';

    const rect = target.getBoundingClientRect();
    const cardRect = this.pointerCard.getBoundingClientRect();

    if (config.placement === 'bottom') {
      this.pointerCard.style.top = `${rect.bottom + 14}px`;
      this.pointerCard.style.left = `${rect.left + rect.width / 2 - cardRect.width / 2}px`;
      this.arrowEl.className = 'callout-arrow arrow-top';
    } else if (config.placement === 'top') {
      this.pointerCard.style.top = `${rect.top - cardRect.height - 14}px`;
      this.pointerCard.style.left = `${rect.left + rect.width / 2 - cardRect.width / 2}px`;
      this.arrowEl.className = 'callout-arrow arrow-bottom';
    } else if (config.placement === 'right') {
      this.pointerCard.style.top = `${rect.top + rect.height / 2 - cardRect.height / 2}px`;
      this.pointerCard.style.left = `${rect.right + 14}px`;
      this.arrowEl.className = 'callout-arrow arrow-left';
    } else {
      this.pointerCard.style.top = `${rect.top + rect.height / 2 - cardRect.height / 2}px`;
      this.pointerCard.style.left = `${rect.left - cardRect.width - 14}px`;
      this.arrowEl.className = 'callout-arrow arrow-right';
    }
  }

  public hide() {
    this.container.style.display = 'none';
  }
}
