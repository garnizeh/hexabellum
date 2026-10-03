import * as PIXI from 'pixi.js';
import {
  initGame,
  getPlayerState,
  getMapHexes,
  getObstacles,
  getPlayerFog,
  restart,
} from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';
import { Animator } from './game/animator';
import { TurnTimer } from './game/timer';

async function main() {
  await initGame();

  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  const app = new PIXI.Application();
  await app.init({
    canvas,
    resizeTo: window,
    backgroundColor: 0x090b14,
    antialias: true,
  });

  const renderer = new HexRenderer(app);
  const animator = new Animator(renderer);
  const timer = new TurnTimer(30);

  const timerEl = document.getElementById('timer') as HTMLElement;
  const roundEl = document.getElementById('round-val') as HTMLElement;
  const statusEl = document.getElementById('status-msg') as HTMLElement;
  const endTurnBtn = document.getElementById('end-turn-btn') as HTMLButtonElement;
  const restartBtn = document.getElementById('restart-btn') as HTMLButtonElement;
  const gameOverModal = document.getElementById('game-over-modal') as HTMLElement;
  const gameOverTitle = document.getElementById('game-over-title') as HTMLElement;
  const modalRestartBtn = document.getElementById('btn-restart-modal') as HTMLButtonElement;

  if (timerEl) timer.setDisplay(timerEl);

  const hexes = getMapHexes();
  const obstacles = getObstacles();

  const refreshBoard = () => {
    const state = getPlayerState(0);
    const fog = getPlayerFog();

    renderer.drawMap(hexes, obstacles);
    renderer.drawUnits(state);
    renderer.drawFog(fog, hexes);

    if (roundEl) roundEl.textContent = `Round ${state.round}`;

    if (state.phase === 'MatchEnd') {
      const won = state.winner === 0;
      const resultText = won ? 'VICTORY — ENEMY BASE DESTROYED' : 'DEFEAT — ALL HEROES ELIMINATED';
      if (statusEl) {
        statusEl.textContent = resultText;
        statusEl.style.color = won ? '#00e676' : '#ff1744';
      }
      timer.stop();
      if (endTurnBtn) endTurnBtn.disabled = true;
      if (restartBtn) restartBtn.style.display = 'inline-block';

      if (gameOverModal && gameOverTitle) {
        gameOverTitle.textContent = won ? 'VICTORY' : 'DEFEAT';
        gameOverTitle.className = won ? 'victory' : 'defeat';
        gameOverModal.style.display = 'flex';
      }
    } else {
      if (gameOverModal) {
        gameOverModal.style.display = 'none';
      }
      if (statusEl) {
        statusEl.textContent = 'Planning Phase — Order Heroes or wait for timer';
        statusEl.style.color = '#00d2ff';
      }
      if (endTurnBtn) endTurnBtn.disabled = false;
      timer.start(() => {
        if (!endTurnBtn.disabled) {
          endTurnBtn.click();
        }
      });
    }
  };

  const input = new InputHandler(renderer);

  const handleEndTurn = () => {
    timer.stop();
    endTurnBtn.disabled = true;
    if (statusEl) statusEl.textContent = 'Resolving simultaneous turn...';
    input.endTurnWithAnimation(animator, () => {
      refreshBoard();
    });
  };

  if (endTurnBtn) {
    endTurnBtn.addEventListener('click', handleEndTurn);
  }

  const handleRestart = () => {
    restart();
    if (restartBtn) restartBtn.style.display = 'none';
    if (gameOverModal) gameOverModal.style.display = 'none';
    refreshBoard();
  };

  if (restartBtn) {
    restartBtn.addEventListener('click', handleRestart);
  }

  if (modalRestartBtn) {
    modalRestartBtn.addEventListener('click', handleRestart);
  }

  const quickResetBtn = document.getElementById('btn-quick-reset') as HTMLButtonElement;
  if (quickResetBtn) {
    quickResetBtn.addEventListener('click', handleRestart);
  }

  refreshBoard();
  console.log("Hexabellum Phase 2 MOBA Vertical Slice Initialized.");
}

main().catch(console.error);
