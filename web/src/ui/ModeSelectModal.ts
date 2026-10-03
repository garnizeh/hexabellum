import { ScenarioLoader } from '../game/tutorial/ScenarioLoader';

export class ModeSelectModal {
  private modalEl: HTMLElement;
  private ftueBannerEl: HTMLElement | null = null;
  private lessonSelectEl: HTMLSelectElement | null = null;

  private onStartTutorialCb: ((lessonId: string) => void) | null = null;
  private onStartPvAICb: (() => void) | null = null;
  private onStartPvPCb: (() => void) | null = null;
  private onJoinMatchCb: ((matchId: string) => void) | null = null;

  constructor() {
    this.modalEl = document.createElement('div');
    this.modalEl.id = 'mode-selection-modal';
    this.renderModal();
    document.body.appendChild(this.modalEl);

    this.renderFtueBanner();
  }

  public setOnStartTutorial(cb: (lessonId: string) => void) {
    this.onStartTutorialCb = cb;
  }

  public setOnStartPvAI(cb: () => void) {
    this.onStartPvAICb = cb;
  }

  public setOnStartPvP(cb: () => void) {
    this.onStartPvPCb = cb;
  }

  public setOnJoinMatch(cb: (matchId: string) => void) {
    this.onJoinMatchCb = cb;
  }

  public open() {
    this.modalEl.style.display = 'flex';
  }

  public close() {
    this.modalEl.style.display = 'none';
  }

  public isOpen(): boolean {
    return this.modalEl.style.display === 'flex';
  }

  public showFtueBanner(onStartTutorial: () => void) {
    if (this.ftueBannerEl) {
      this.ftueBannerEl.style.display = 'flex';
      const btn = this.ftueBannerEl.querySelector('#btn-ftue-start') as HTMLButtonElement;
      if (btn) {
        btn.onclick = () => {
          this.hideFtueBanner();
          onStartTutorial();
        };
      }
    }
  }

  public hideFtueBanner() {
    if (this.ftueBannerEl) {
      this.ftueBannerEl.style.display = 'none';
      try {
        localStorage.setItem('hexabellum_ftue_banner_dismissed', '1');
      } catch {}
    }
  }

  private renderModal() {
    const lessons = ScenarioLoader.getAllLessons();
    const lessonOptions = lessons
      .map(l => `<option value="${l.id}">Lição ${l.index}: ${l.title}</option>`)
      .join('');

    this.modalEl.innerHTML = `
      <div class="mode-hub-box glass-panel">
        <div class="mode-hub-header">
          <h2 class="mode-hub-title">HEXABELLUM — PORTAL DA CONVERGÊNCIA</h2>
          <p class="mode-hub-subtitle">Turnos Simultâneos. Determinismo Absoluto. Seis Lados de Guerra.</p>
        </div>

        <div class="mode-cards-grid">
          <!-- Card 1: Tutorial Proving Grounds -->
          <div class="mode-card mode-card-tutorial">
            <div>
              <div class="mode-badge badge-rec">✦ Recomendado para Iniciantes</div>
              <h3 class="mode-card-title">🎓 O Julgamento do Archmage</h3>
              <p class="mode-card-desc">
                Nível guiado single-player passo a passo. Aprenda as regras fazendo, sem pressão de tempo e com mentoria pedagógica.
              </p>

              <ul class="mode-feature-list">
                <li class="mode-feature-item">
                  <span class="mode-feature-icon">🧭</span>
                  <span><strong>Os Três Passos:</strong> Movimentação hexagonal, custo de 1 AP/hex e colisão.</span>
                </li>
                <li class="mode-feature-item">
                  <span class="mode-feature-icon">⚡</span>
                  <span><strong>Iniciativa & Morte:</strong> Ordem de resolução e regra <em>Death-Before-Acting</em>.</span>
                </li>
                <li class="mode-feature-item">
                  <span class="mode-feature-icon">🧙</span>
                  <span><strong>Mentoria Archmage:</strong> Diálogos inteligentes e feedback construtivo (Soft-Fail).</span>
                </li>
              </ul>

              <div class="lesson-selector-row">
                <label for="hub-lesson-select">Pular para Lição Específica:</label>
                <select id="hub-lesson-select" class="lesson-select-dropdown">
                  ${lessonOptions}
                </select>
              </div>
            </div>

            <button id="btn-hub-start-tutorial" class="btn-mode-start btn-mode-tutorial">
              <span>Iniciar Julgamento do Archmage</span> ➔
            </button>
          </div>

          <!-- Card 2: Standard Game (Fractured Meridian) -->
          <div class="mode-card mode-card-arena">
            <div>
              <div class="mode-badge badge-arena">⚔️ Arena Oficial</div>
              <h3 class="mode-card-title">⚔️ Jogo Comum (Fractured Meridian)</h3>
              <p class="mode-card-desc">
                A experiência tática completa: 3v3 MOBA com avanço de rotas, tropas automáticas, torres defensivas e destruição de base.
              </p>

              <ul class="mode-feature-list">
                <li class="mode-feature-item">
                  <span class="mode-feature-icon">🤖</span>
                  <span><strong>Partida vs IA:</strong> Enfrente a inteligência tática do servidor autoritativo em tempo real.</span>
                </li>
                <li class="mode-feature-item">
                  <span class="mode-feature-icon">🌐</span>
                  <span><strong>Duelo 1v1 PvP:</strong> Convide um oponente online com link direto da sala.</span>
                </li>
                <li class="mode-feature-item">
                  <span class="mode-feature-icon">🏰</span>
                  <span><strong>Destruição de Base:</strong> Empurre as rotas e destrua as torres para alcançar a vitória.</span>
                </li>
              </ul>

              <div class="modal-btn-group" style="gap: 8px;">
                <button id="btn-hub-start-pvai" class="btn-mode-start btn-mode-pvai">
                  🤖 Jogar contra Servidor IA (PvAI)
                </button>
                <button id="btn-hub-start-pvp" class="btn-mode-pvp-sub">
                  ⚔️ Criar Sala 1v1 PvP Online
                </button>
                <div class="join-row" style="margin-top: 2px;">
                  <input id="input-hub-join-id" class="join-input" placeholder="ID da Partida..." />
                  <button id="btn-hub-submit-join" class="btn-action-small">Entrar</button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="mode-hub-footer">
          <button id="btn-hub-close" class="btn-action-small" style="padding: 8px 24px; font-size: 13px;">
            Continuar no Jogo Atual
          </button>
        </div>
      </div>
    `;

    this.lessonSelectEl = this.modalEl.querySelector('#hub-lesson-select') as HTMLSelectElement;

    this.modalEl.querySelector('#btn-hub-start-tutorial')?.addEventListener('click', () => {
      const selected = this.lessonSelectEl?.value || 'lesson_00_intro';
      this.close();
      if (this.onStartTutorialCb) this.onStartTutorialCb(selected);
    });

    this.modalEl.querySelector('#btn-hub-start-pvai')?.addEventListener('click', () => {
      this.close();
      if (this.onStartPvAICb) this.onStartPvAICb();
    });

    this.modalEl.querySelector('#btn-hub-start-pvp')?.addEventListener('click', () => {
      this.close();
      if (this.onStartPvPCb) this.onStartPvPCb();
    });

    this.modalEl.querySelector('#btn-hub-submit-join')?.addEventListener('click', () => {
      const input = this.modalEl.querySelector('#input-hub-join-id') as HTMLInputElement;
      if (input && input.value.trim()) {
        this.close();
        if (this.onJoinMatchCb) this.onJoinMatchCb(input.value.trim());
      }
    });

    this.modalEl.querySelector('#btn-hub-close')?.addEventListener('click', () => {
      this.close();
    });
  }

  private renderFtueBanner() {
    this.ftueBannerEl = document.createElement('div');
    this.ftueBannerEl.id = 'ftue-welcome-banner';
    this.ftueBannerEl.style.display = 'none';

    this.ftueBannerEl.innerHTML = `
      <div class="ftue-banner-text">
        🧙 <strong>Novo em Hexabellum?</strong> Domine movimento e iniciativa no <em>Julgamento do Archmage</em>!
      </div>
      <button id="btn-ftue-start" class="btn-banner-start">Jogar Tutorial</button>
      <button id="btn-ftue-dismiss" class="btn-banner-dismiss" title="Fechar">✕</button>
    `;

    document.body.appendChild(this.ftueBannerEl);

    this.ftueBannerEl.querySelector('#btn-ftue-dismiss')?.addEventListener('click', () => {
      this.hideFtueBanner();
    });
  }
}
