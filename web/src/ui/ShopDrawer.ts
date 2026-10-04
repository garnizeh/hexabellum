import { ClientSession } from '../game/client_session';
import { ItemDto, HeroEconomyDto } from '../game/types';

export class ShopDrawer {
  private overlayEl: HTMLElement | null = null;
  private drawerEl: HTMLElement | null = null;
  private goldDisplayEl: HTMLElement | null = null;
  private itemsContainerEl: HTMLElement | null = null;
  private session: ClientSession | null = null;
  private isOpenState = false;

  private canonicalItems: ItemDto[] = [
    {
      id: 'longblade',
      name: 'Longblade',
      cost: 100,
      description: 'Tempered high-carbon steel blade. Increases attack damage by 6.',
      modifiers: [{ stat: 'AttackDamage', value: 6 }],
    },
    {
      id: 'plate_armor',
      name: 'Plate Armor',
      cost: 120,
      description: 'Reinforced cuirass. Increases maximum health and current health by 35.',
      modifiers: [{ stat: 'MaxHealth', value: 35 }],
    },
    {
      id: 'scout_lens',
      name: 'Scout Lens',
      cost: 80,
      description: 'Convex optical lens. Expands sight radius into the fog of war by 1 hex.',
      modifiers: [{ stat: 'VisionRange', value: 1 }],
    },
    {
      id: 'focus_charm',
      name: 'Focus Charm',
      cost: 100,
      description: 'Inscribed channel talisman. Restores 1 additional energy at round start.',
      modifiers: [{ stat: 'EnergyRegen', value: 1 }],
    },
  ];

  constructor() {
    this.createDom();
    this.setupKeyboardListeners();
  }

  setSession(session: ClientSession | null): void {
    this.session = session;
    if (this.isOpenState) {
      this.render();
    }
  }

  private createDom(): void {
    if (document.getElementById('hb-shop-drawer')) {
      this.drawerEl = document.getElementById('hb-shop-drawer');
      this.overlayEl = document.getElementById('hb-shop-overlay');
      return;
    }

    const overlay = document.createElement('div');
    overlay.id = 'hb-shop-overlay';
    overlay.className = 'hb-shop-drawer-overlay';
    document.body.appendChild(overlay);
    this.overlayEl = overlay;

    const drawer = document.createElement('div');
    drawer.id = 'hb-shop-drawer';
    drawer.className = 'hb-shop-drawer';

    drawer.innerHTML = `
      <div class="hb-shop-header">
        <div class="hb-shop-title-group">
          <div class="hb-shop-title">
            <span>🛡️ FIELD SHOP</span>
            <span class="badge">PLANNING</span>
          </div>
        </div>
        <div style="display:flex; align-items:center; gap:10px;">
          <div id="hb-shop-gold" class="hb-shop-gold-display">🪙 0 G</div>
          <button id="hb-shop-close-btn" class="hb-shop-close-btn" title="Close [Esc]">✕</button>
        </div>
      </div>
      <div id="hb-shop-items-container" class="hb-shop-body"></div>
      <div class="hb-shop-footer">
        <span>Hotkey: <strong>[B]</strong> to toggle &nbsp;|&nbsp; <strong>[1-4]</strong> quick-buy</span>
        <span>Slots: 3 Max</span>
      </div>
    `;

    document.body.appendChild(drawer);
    this.drawerEl = drawer;
    this.goldDisplayEl = drawer.querySelector('#hb-shop-gold');
    this.itemsContainerEl = drawer.querySelector('#hb-shop-items-container');

    overlay.addEventListener('click', () => this.close());
    drawer.querySelector('#hb-shop-close-btn')?.addEventListener('click', () => this.close());
  }

  private setupKeyboardListeners(): void {
    window.addEventListener('keydown', (e: KeyboardEvent) => {
      // Ignore if user is typing in an input
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      if (e.key === 'b' || e.key === 'B') {
        e.preventDefault();
        this.toggle();
        return;
      }

      if (e.key === 'Escape' && this.isOpenState) {
        e.preventDefault();
        this.close();
        return;
      }

      if (this.isOpenState && ['1', '2', '3', '4'].includes(e.key)) {
        const index = parseInt(e.key, 10) - 1;
        this.attemptQuickBuy(index);
      }
    });
  }

  toggle(): void {
    if (this.isOpenState) {
      this.close();
    } else {
      this.open();
    }
  }

  open(): void {
    this.isOpenState = true;
    this.overlayEl?.classList.add('open');
    this.drawerEl?.classList.add('open');
    this.render();
  }

  close(): void {
    this.isOpenState = false;
    this.overlayEl?.classList.remove('open');
    this.drawerEl?.classList.remove('open');
  }

  isOpen(): boolean {
    return this.isOpenState;
  }

  update(): void {
    if (this.isOpenState) {
      this.render();
    }
  }

  private getControlledEconomy(): { gold: number; items: string[] } {
    if (!this.session) return { gold: 0, items: [] };

    const eco = this.session.getControlledHeroEconomy();
    if (eco) {
      return { gold: eco.gold, items: eco.items };
    }

    // Fallback: search hero in snapshot units
    const snapshot = (this.session as any).currentSnapshot;
    if (snapshot) {
      const myUnitId = snapshot.controlled_units?.[0];
      const unit = snapshot.units?.find((u: any) => u.id === myUnitId);
      if (unit) {
        return { gold: unit.gold ?? 0, items: unit.items ?? [] };
      }
    }

    return { gold: 0, items: [] };
  }

  private attemptQuickBuy(index: number): void {
    const items = this.getItemsCatalog();
    if (index >= 0 && index < items.length) {
      const item = items[index];
      const eco = this.getControlledEconomy();
      const isPlanning = this.session?.canShop() ?? false;

      if (!isPlanning) return;
      if (eco.items.includes(item.id)) return;
      if (eco.items.length >= 3) return;
      if (eco.gold < item.cost) return;

      this.session?.buyItem(item.id);
    }
  }

  private getItemsCatalog(): ItemDto[] {
    const serverCatalog = this.session?.getShopCatalog();
    if (serverCatalog && serverCatalog.length > 0) {
      return serverCatalog;
    }
    return this.canonicalItems;
  }

  private getItemIcon(itemId: string): string {
    switch (itemId) {
      case 'longblade': return '⚔️';
      case 'plate_armor': return '🛡️';
      case 'scout_lens': return '👁️';
      case 'focus_charm': return '🔮';
      default: return '📦';
    }
  }

  render(): void {
    if (!this.itemsContainerEl) return;

    const eco = this.getControlledEconomy();
    const canShop = this.session?.canShop() ?? false;
    const items = this.getItemsCatalog();

    if (this.goldDisplayEl) {
      this.goldDisplayEl.textContent = `🪙 ${eco.gold} G`;
    }

    let html = '';

    items.forEach((item, index) => {
      const hotkey = index + 1;
      const isOwned = eco.items.includes(item.id);
      const isBagFull = eco.items.length >= 3;
      const canAfford = eco.gold >= item.cost;

      let btnClass = 'hb-shop-buy-btn';
      let btnText = `BUY (${item.cost} G)`;
      let btnDisabled = false;

      if (isOwned) {
        btnClass += ' owned';
        btnText = '✓ OWNED';
        btnDisabled = true;
      } else if (!canShop) {
        btnClass += ' phase-locked';
        const reason = this.session?.getShopDisabledReason();
        if (reason === 'outside_base_zone') {
          btnText = 'MUST BE IN BASE';
        } else if (reason === 'hero_dead') {
          btnText = 'HERO DEFEATED';
        } else {
          btnText = 'PLANNING ONLY';
        }
        btnDisabled = true;
      } else if (isBagFull) {
        btnClass += ' inventory-full';
        btnText = 'BAG FULL (3/3)';
        btnDisabled = true;
      } else if (!canAfford) {
        btnClass += ' cant-afford';
        btnText = `NEED ${item.cost - eco.gold} MORE G`;
        btnDisabled = true;
      } else {
        btnClass += ' can-buy';
        btnText = `BUY [${hotkey}] (${item.cost} G)`;
      }

      const statBadges = item.modifiers
        .map((m) => {
          const statLabel = this.formatStatName(m.stat);
          return `<span class="hb-shop-stat-badge">+${m.value} ${statLabel}</span>`;
        })
        .join('');

      html += `
        <div class="hb-shop-item-card ${isOwned ? 'owned-card' : ''}">
          <div class="hb-shop-item-card-top">
            <div class="hb-shop-item-icon-frame">
              ${this.getItemIcon(item.id)}
            </div>
            <div class="hb-shop-item-meta">
              <div class="hb-shop-item-name">
                <span>${item.name}</span>
                <span class="hb-shop-hotkey-pip">[${hotkey}]</span>
              </div>
              <div class="hb-shop-item-cost">🪙 ${item.cost} Gold</div>
            </div>
          </div>
          <div class="hb-shop-item-desc">${item.description}</div>
          <div class="hb-shop-item-stats">${statBadges}</div>
          <button class="${btnClass}" data-item-id="${item.id}" ${btnDisabled ? 'disabled' : ''}>
            <span>${btnText}</span>
            <span class="hb-shop-hotkey-pip">#${hotkey}</span>
          </button>
        </div>
      `;
    });

    this.itemsContainerEl.innerHTML = html;

    // Attach buy click handlers
    const buttons = this.itemsContainerEl.querySelectorAll<HTMLButtonElement>('.hb-shop-buy-btn.can-buy');
    buttons.forEach((btn) => {
      btn.addEventListener('click', () => {
        const itemId = btn.getAttribute('data-item-id');
        if (itemId && this.session) {
          this.session.buyItem(itemId);
        }
      });
    });
  }

  private formatStatName(stat: string): string {
    switch (stat) {
      case 'AttackDamage': return 'Attack Dmg';
      case 'MaxHealth':
      case 'MaxHp': return 'Max HP';
      case 'EnergyRegen': return 'Energy Regen';
      case 'MaxEnergy': return 'Max Energy';
      case 'MaxAp': return 'Max AP';
      case 'VisionRange': return 'Vision';
      case 'Initiative': return 'Initiative';
      default: return stat;
    }
  }
}
