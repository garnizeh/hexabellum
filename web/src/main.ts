import { initGame, getState, getMapHexes, getObstacles } from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';

async function main() {
  // Initialize WASM
  await initGame();

  // Create canvas
  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;

  // Create renderer
  const renderer = new HexRenderer(canvas);

  // Draw initial state
  const hexes = getMapHexes();
  const obstacles = getObstacles();
  const state = getState();

  renderer.drawMap(hexes, obstacles);
  renderer.drawUnits(state);

  // Create input handler
  const input = new InputHandler(renderer);

  // HUD elements
  const endTurnBtn = document.getElementById('end-turn') as HTMLButtonElement;
  const roundDisplay = document.getElementById('round') as HTMLElement;
  const selectionDisplay = document.getElementById('selection') as HTMLElement;

  const updateHud = () => {
    const currentState = getState();
    roundDisplay.textContent = `Round ${currentState.round}`;
    const sel = input.getSelectedUnit();
    selectionDisplay.textContent = sel !== null ? `Selected: Hero #${sel}` : '';
  };

  endTurnBtn.addEventListener('click', () => {
    input.endTurn();
    updateHud();
  });

  updateHud();

  console.log("Hexabellum Phase 0 initialized!");
}

main().catch(console.error);
