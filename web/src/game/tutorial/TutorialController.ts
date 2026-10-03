import * as PIXI from 'pixi.js';
import { HexRenderer } from '../renderer';
import { Animator } from '../animator';
import { createTutorialSession, GameState, HexCoord } from '../bridge';
import { WasmTutorialSession } from '../../wasm/pkg/hexabellum_wasm';
import { ScenarioLoader, TutorialScenario, TutorialStep } from './ScenarioLoader';
import { WizardDialogueCard } from './ui/WizardDialogueCard';
import { ObjectiveChecklist } from './ui/ObjectiveChecklist';
import { CalloutOverlay } from './ui/CalloutOverlay';
import { SpotlightRenderer } from './SpotlightRenderer';
import { InputGateFilter } from './InputGateFilter';
import { AutoDemoPlayer } from './AutoDemoPlayer';
import { TutorialSound } from './TutorialSound';

export interface TutorialPersistenceSchema {
  version: 1;
  highest_completed_lesson: number;
  is_graduation_completed: boolean;
  quiet_mode_enabled: boolean;
}

const STORAGE_KEY = 'hexabellum_tutorial_v1';

export class TutorialController {
  private app: PIXI.Application;
  private renderer: HexRenderer;
  private animator: Animator;

  private session: WasmTutorialSession | null = null;
  private currentScenario: TutorialScenario | null = null;
  private currentStepDef: TutorialStep | null = null;

  private dialogueCard: WizardDialogueCard;
  private checklist: ObjectiveChecklist;
  private callout: CalloutOverlay;
  private spotlight: SpotlightRenderer;
  private inputGate: InputGateFilter;
  private autoDemo: AutoDemoPlayer;

  private antiStuckTimer: number | null = null;
  private stepStartTime: number = 0;
  private isQuietMode: boolean = false;
  private isTrialActive: boolean = false;

  private onExitCallback: (() => void) | null = null;
  private drawerModal: HTMLElement | null = null;
  private graduationModal: HTMLElement | null = null;

  constructor(app: PIXI.Application, renderer: HexRenderer, animator: Animator) {
    this.app = app;
    this.renderer = renderer;
    this.animator = animator;

    this.dialogueCard = new WizardDialogueCard();
    this.checklist = new ObjectiveChecklist();
    this.callout = new CalloutOverlay();
    this.spotlight = new SpotlightRenderer(app);
    this.inputGate = new InputGateFilter();
    this.autoDemo = new AutoDemoPlayer();

    this.loadPersistence();
    this.buildDrawerModal();
    this.buildGraduationModal();

    this.checklist.setOnMenuClick(() => {
      this.toggleDrawer();
    });

    window.addEventListener('keydown', (e) => {
      if (this.isTrialActive && e.key === 'Escape') {
        e.preventDefault();
        this.toggleDrawer();
      }
    });
  }

  public setOnExit(cb: () => void) {
    this.onExitCallback = cb;
  }

  public isRunning(): boolean {
    return this.isTrialActive;
  }

  public startLesson(lessonId: string = 'lesson_00_intro') {
    this.isTrialActive = true;
    this.currentScenario = ScenarioLoader.getScenario(lessonId);

    try {
      this.session = createTutorialSession(lessonId);
    } catch (err) {
      console.warn("WASM Tutorial Session init failed, using JSON string:", err);
      this.session = createTutorialSession(JSON.stringify(this.currentScenario));
    }

    this.checklist.show();
    this.renderBoard();
    this.loadActiveStep();

    // Update URL without full page reload
    if (typeof window !== 'undefined' && window.history) {
      const url = new URL(window.location.href);
      url.searchParams.set('mode', 'tutorial');
      url.searchParams.set('lesson', lessonId);
      window.history.replaceState(null, '', url.toString());
    }
  }

  public exitTutorial() {
    this.isTrialActive = false;
    this.stopAntiStuck();
    this.dialogueCard.hide();
    this.checklist.hide();
    this.callout.hide();
    this.spotlight.clearSpotlights();
    this.autoDemo.stop();
    if (this.drawerModal) this.drawerModal.style.display = 'none';

    if (typeof window !== 'undefined' && window.history) {
      const url = new URL(window.location.href);
      url.searchParams.delete('lesson');
      window.history.replaceState(null, '', url.toString());
    }

    if (this.onExitCallback) {
      this.onExitCallback();
    }
  }

  public handleCanvasClick(pixelX: number, pixelY: number): boolean {
    if (!this.isTrialActive || !this.currentStepDef) return false;

    // Convert pixel coordinates to axial hex
    const { q, r } = this.pixelToHex(pixelX, pixelY);
    const state = this.getCurrentState();
    const playerHero = Object.values(state.units).find(u => u.team === 0);
    const targetUnit = Object.values(state.units).find(u => u.pos.q === q && u.pos.r === r);

    const canAttack = this.currentStepDef.input_gate?.allowed_actions.some(
      a => a.toLowerCase() === 'attack'
    );

    // If unit clicked and attack action is permitted: process attack order
    if (canAttack && targetUnit && targetUnit.team !== 0) {
      const validation = this.inputGate.validateAttack(playerHero ? playerHero.id : 1, targetUnit.id);
      if (!validation.allowed) {
        this.dialogueCard.showSoftFail(validation.errorMessage || 'Invalid strike target.');
        return true;
      }
      this.submitOrder(playerHero ? playerHero.id : 1, null, null, targetUnit.id);
      return true;
    }

    // Move action (handles moving into empty hex or testing collision against occupied hex)
    const validation = this.inputGate.validateMove(playerHero ? playerHero.id : 1, q, r);
    if (!validation.allowed) {
      this.dialogueCard.showSoftFail(validation.errorMessage || 'Hex is outside current lesson objective.');
      return true;
    }

    this.submitOrder(playerHero ? playerHero.id : 1, q, r, null);
    return true;
  }

  public submitOrder(unitId: number, q: number | null, r: number | null, targetId: number | null) {
    if (!this.session) return;

    const resJson = this.session.submit_order(
      BigInt(unitId),
      q !== null ? q : undefined,
      r !== null ? r : undefined,
      targetId !== null ? BigInt(targetId) : undefined
    );

    const res = JSON.parse(resJson);
    if (!res.valid) {
      this.dialogueCard.showSoftFail(res.archmage_feedback || 'The grid rejects this order.');
      return;
    }

    this.renderBoard();

    // Check if this step is an Attack step requiring round resolution
    if (this.currentStepDef && this.currentStepDef.completion_condition.condition === 'RoundResolved') {
      const eventsJson = this.session.resolve_round();
      try {
        const events = JSON.parse(eventsJson);
        this.animator.playEvents(events, () => {
          this.renderBoard();
          this.handleStepCompletion();
        });
        return;
      } catch {
        this.renderBoard();
        this.handleStepCompletion();
      }
    }

    if (res.completed_step) {
      this.handleStepCompletion();
    }
  }

  private handleStepCompletion(isDialogue: boolean = false) {
    TutorialSound.playStepComplete();
    this.checklist.markCompleted();

    const delay = isDialogue ? 40 : 600;
    setTimeout(() => {
      if (this.session && this.session.is_completed()) {
        this.handleLessonCompleted();
      } else {
        this.loadActiveStep();
      }
    }, delay);
  }

  private loadActiveStep() {
    if (!this.session || !this.currentScenario) return;

    this.stopAntiStuck();
    this.callout.hide();
    this.spotlight.clearSpotlights();

    if (this.session.is_completed()) {
      this.handleLessonCompleted();
      return;
    }

    const stepId = this.session.get_current_step_id();
    const stepIdx = this.currentScenario.steps.findIndex(s => s.step_id === stepId);
    if (stepIdx === -1) return;

    this.currentStepDef = this.currentScenario.steps[stepIdx];
    const totalSteps = this.currentScenario.steps.length;

    // 1. Update checklist text
    let taskDesc = 'Follow Archmage guidance';
    if (this.currentStepDef.objective) {
      taskDesc = this.currentStepDef.objective.text;
    } else if (this.currentStepDef.dialogue) {
      taskDesc = 'Listen to the Archmage';
    }
    this.checklist.updateLesson(this.currentScenario.title, stepIdx, totalSteps, taskDesc);

    // 2. Configure input gate
    this.inputGate.setGate(
      this.currentStepDef.input_gate,
      this.currentStepDef.failure_feedback || []
    );

    // 3. Highlight hexes
    if (this.currentStepDef.objective && this.currentStepDef.objective.spotlight_hexes.length > 0) {
      this.spotlight.setSpotlights(
        this.currentStepDef.objective.spotlight_hexes,
        this.currentStepDef.objective.ring_color
      );
    }

    // 4. Callout overlay
    if (this.currentStepDef.callout) {
      this.callout.showCallout(this.currentStepDef.callout);
    }

    // 5. Dialogue presentation
    if (this.currentStepDef.dialogue && !this.isQuietMode) {
      this.dialogueCard.showDialogue(
        this.currentStepDef.dialogue.lines,
        this.currentStepDef.dialogue.emotion,
        () => {
          this.callout.hide();
          if (this.session && this.currentStepDef?.completion_condition.condition === 'DialogueDismissed') {
            this.session.advance_dialogue();
            this.handleStepCompletion(true);
          }
        },
        () => {
          this.skipStep();
        }
      );
    }

    // 6. Anti-stuck escalation ladder (Stage 1 -> Stage 2 -> Stage 3)
    if (this.currentStepDef.step_type === 'Objective') {
      this.startAntiStuckLadder();
    }
  }

  private startAntiStuckLadder() {
    this.stepStartTime = Date.now();
    this.antiStuckTimer = window.setInterval(() => {
      const elapsed = (Date.now() - this.stepStartTime) / 1000;
      if (elapsed >= 90) {
        // Stage 3: Auto-demo demonstration button
        this.dialogueCard.showAutoDemoButton(() => {
          this.executeAutoDemo();
        });
      } else if (elapsed >= 45) {
        // Stage 2: Explicit hint in dialogue
        if (this.currentStepDef?.objective && !this.dialogueCard.isVisible()) {
          const hex = this.currentStepDef.objective.spotlight_hexes[0];
          const hint = hex
            ? `Notice the glowing target at coordinate (${hex.q}, ${hex.r}). Click that stone to complete your task.`
            : 'Observe the highlighted area and issue your command.';
          this.dialogueCard.showDialogue([hint], 'Neutral', () => {});
        }
      }
    }, 5000);
  }

  private stopAntiStuck() {
    if (this.antiStuckTimer !== null) {
      clearInterval(this.antiStuckTimer);
      this.antiStuckTimer = null;
    }
    this.dialogueCard.hideAutoDemoButton();
  }

  private executeAutoDemo() {
    const state = this.getCurrentState();
    const hero = Object.values(state.units).find(u => u.team === 0);
    const targetHex = this.currentStepDef?.objective?.spotlight_hexes[0];
    if (!hero || !targetHex) return;

    const fromPixel = this.spotlight.hexToPixel(hero.pos.q, hero.pos.r);
    const toPixel = this.spotlight.hexToPixel(targetHex.q, targetHex.r);

    this.autoDemo.playDemo(fromPixel, toPixel, () => {
      this.renderBoard();
    });
  }

  private skipStep() {
    if (!this.session || !this.currentScenario) return;
    this.session.advance_dialogue();
    this.handleStepCompletion();
  }

  private handleLessonCompleted() {
    this.stopAntiStuck();
    this.spotlight.clearSpotlights();
    this.checklist.hide();

    const currIdx = this.currentScenario?.lesson_index || 0;
    this.saveProgress(currIdx);

    const allLessons = ScenarioLoader.getAllLessons();
    const nextLesson = allLessons.find(l => l.index > currIdx);

    if (nextLesson) {
      this.dialogueCard.showDialogue(
        [
          `Trial Lesson ${currIdx} Concluded: ${this.currentScenario?.title}.`,
          `Honor to your strategy! Next lesson awaits: ${nextLesson.title}.`,
        ],
        'Pleased',
        () => {
          this.startLesson(nextLesson.id);
        }
      );
    } else {
      // All lessons complete -> Graduation Modal!
      TutorialSound.playVictoryFanfare();
      this.showGraduationModal();
    }
  }

  private renderBoard() {
    if (!this.session) return;
    const state: GameState = JSON.parse(this.session.get_state());
    const hexes: HexCoord[] = JSON.parse(this.session.get_map_hexes()).map(([q, r]: [number, number]) => ({ q, r }));
    const obstacles: HexCoord[] = JSON.parse(this.session.get_obstacles()).map(([q, r]: [number, number]) => ({ q, r }));
    const fog: HexCoord[] = JSON.parse(this.session.get_player_fog()).map(([q, r]: [number, number]) => ({ q, r }));

    this.renderer.setPlayerTeam(0);
    this.renderer.drawMap(hexes, obstacles);
    this.renderer.drawUnits(state);
    this.renderer.drawFog(fog, hexes);
  }

  private getCurrentState(): GameState {
    if (!this.session) {
      return { round: 0, phase: 'Planning', units: {}, winner: null };
    }
    return JSON.parse(this.session.get_state());
  }

  private pixelToHex(x: number, y: number): { q: number; r: number } {
    const cx = x - this.app.screen.width / 2;
    const cy = y - this.app.screen.height / 2;
    const size = 30; // HEX_SIZE
    const q = ((Math.sqrt(3) / 3) * cx - (1 / 3) * cy) / size;
    const r = ((2 / 3) * cy) / size;
    return this.hexRound(q, r);
  }

  private hexRound(q: number, r: number): { q: number; r: number } {
    const s = -q - r;
    let rq = Math.round(q);
    let rr = Math.round(r);
    let rs = Math.round(s);

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

  // --- Persistence & Storage ---

  private loadPersistence(): TutorialPersistenceSchema {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        this.isQuietMode = !!parsed.quiet_mode_enabled;
        return parsed;
      }
    } catch (e) {
      console.warn("Corrupted tutorial storage, resetting defaults:", e);
    }
    return {
      version: 1,
      highest_completed_lesson: -1,
      is_graduation_completed: false,
      quiet_mode_enabled: false,
    };
  }

  private saveProgress(completedLessonIdx: number) {
    const curr = this.loadPersistence();
    curr.highest_completed_lesson = Math.max(curr.highest_completed_lesson, completedLessonIdx);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(curr));
  }

  // --- Drawer & Graduation Modals ---

  private toggleDrawer() {
    if (!this.drawerModal) return;
    if (this.drawerModal.style.display === 'flex') {
      this.drawerModal.style.display = 'none';
    } else {
      this.drawerModal.style.display = 'flex';
    }
  }

  private buildDrawerModal() {
    this.drawerModal = document.createElement('div');
    this.drawerModal.id = 'tutorial-nav-drawer';
    this.drawerModal.className = 'drawer-modal';
    this.drawerModal.style.display = 'none';

    this.drawerModal.innerHTML = `
      <div class="drawer-panel glass-panel">
        <h3>TUTORIAL TRIAL MENU</h3>
        <p class="drawer-desc">Adjust pacing or navigate directly to specific drills.</p>
        <div class="drawer-btn-group">
          <button id="btn-drawer-resume" class="btn-drawer primary">Resume Trial</button>
          <button id="btn-drawer-restart" class="btn-drawer">Restart Lesson</button>
          <button id="btn-drawer-skip" class="btn-drawer">Skip to Next Lesson</button>
          <button id="btn-drawer-quiet" class="btn-drawer">Toggle Quiet Mode (${this.isQuietMode ? 'ON' : 'OFF'})</button>
          <button id="btn-drawer-grad" class="btn-drawer">Skip to Graduation Match</button>
          <button id="btn-drawer-exit" class="btn-drawer danger">Exit to Main Arena (Jogo Comum)</button>
        </div>
      </div>
    `;

    document.body.appendChild(this.drawerModal);

    this.drawerModal.querySelector('#btn-drawer-resume')?.addEventListener('click', () => {
      this.toggleDrawer();
    });

    this.drawerModal.querySelector('#btn-drawer-restart')?.addEventListener('click', () => {
      this.toggleDrawer();
      if (this.currentScenario) {
        this.startLesson(this.currentScenario.id);
      }
    });

    this.drawerModal.querySelector('#btn-drawer-skip')?.addEventListener('click', () => {
      this.toggleDrawer();
      this.handleLessonCompleted();
    });

    this.drawerModal.querySelector('#btn-drawer-quiet')?.addEventListener('click', (e) => {
      this.isQuietMode = !this.isQuietMode;
      const target = e.currentTarget as HTMLButtonElement;
      target.textContent = `Toggle Quiet Mode (${this.isQuietMode ? 'ON' : 'OFF'})`;
    });

    this.drawerModal.querySelector('#btn-drawer-grad')?.addEventListener('click', () => {
      this.toggleDrawer();
      this.showGraduationModal();
    });

    this.drawerModal.querySelector('#btn-drawer-exit')?.addEventListener('click', () => {
      this.exitTutorial();
    });
  }

  private showGraduationModal() {
    if (this.graduationModal) {
      this.graduationModal.style.display = 'flex';
    }
  }

  private buildGraduationModal() {
    this.graduationModal = document.createElement('div');
    this.graduationModal.id = 'tutorial-graduation-modal';
    this.graduationModal.className = 'graduation-modal';
    this.graduationModal.style.display = 'none';

    this.graduationModal.innerHTML = `
      <div class="modal-box graduation-box">
        <div class="grad-sigil">🎓</div>
        <h2>THE PROVING GROUNDS CONQUERED!</h2>
        <p class="grad-speech">
          "You have mastered the six sides of war: Movement, Initiative, Preview, and Resolution.
          The Proving Grounds hold no more secrets for you. The true battle of Fractured Meridian begins now."
        </p>
        <div class="modal-btn-group">
          <button id="btn-grad-pvai" class="modal-btn btn-pvai">🤖 Enter the Bellum Sextum (PvAI Match)</button>
          <button id="btn-grad-pvp" class="modal-btn btn-pvp">⚔️ Battle Real Players Online (1v1 PvP)</button>
          <button id="btn-grad-review" class="btn-action-small" style="justify-content: center; margin-top: 10px;">Replay Tutorial</button>
        </div>
      </div>
    `;

    document.body.appendChild(this.graduationModal);

    this.graduationModal.querySelector('#btn-grad-pvai')?.addEventListener('click', () => {
      this.graduationModal!.style.display = 'none';
      this.exitTutorial();
    });

    this.graduationModal.querySelector('#btn-grad-pvp')?.addEventListener('click', () => {
      this.graduationModal!.style.display = 'none';
      this.exitTutorial();
      if (typeof window !== 'undefined' && window.history) {
        const url = new URL(window.location.href);
        url.searchParams.set('mode', 'pvp');
        window.history.replaceState(null, '', url.toString());
      }
    });

    this.graduationModal.querySelector('#btn-grad-review')?.addEventListener('click', () => {
      this.graduationModal!.style.display = 'none';
      this.startLesson('lesson_00_intro');
    });
  }
}
