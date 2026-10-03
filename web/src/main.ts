import * as PIXI from 'pixi.js';
import { initGame, getState, getMapHexes, getObstacles } from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';

async function bootstrap() {
  await initGame();

  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  const app = new PIXI.Application();
  await app.init({
    canvas,
    resizeTo: window,
    backgroundColor: 0x0a0e17,
    antialias: true,
  });

  const renderer = new HexRenderer(app);
  const input = new InputHandler(renderer);

  // Initial draw
  const hexes = getMapHexes();
  const obstacles = getObstacles();
  const state = getState();

  renderer.drawMap(hexes, obstacles);
  renderer.drawUnits(state);

  // Wire UI buttons
  const endTurnBtn = document.getElementById('end-turn') as HTMLButtonElement;
  if (endTurnBtn) {
    endTurnBtn.addEventListener('click', () => input.endTurn());
  }

  const restartBtn = document.getElementById('btn-restart-modal') as HTMLButtonElement;
  if (restartBtn) {
    restartBtn.addEventListener('click', () => input.restartGame());
  }

  const resetBtn = document.getElementById('btn-quick-reset') as HTMLButtonElement;
  if (resetBtn) {
    resetBtn.addEventListener('click', () => input.restartGame());
  }

  console.log('Hexabellum Phase 1 Initialized');
}

bootstrap().catch(console.error);
