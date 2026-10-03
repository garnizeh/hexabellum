import * as PIXI from 'pixi.js';
import {
  initGame,
  getPlayerState,
  getMapHexes,
  getObstacles,
  getPlayerFog,
  restart as restartLocalGame,
} from './game/bridge';
import { HexRenderer } from './game/renderer';
import { InputHandler } from './game/input';
import { Animator } from './game/animator';
import { TurnTimer } from './game/timer';
import { ClientSession, snapshotToGameState } from './game/client_session';
import { HudController } from './ui/hud';
import { SanitizedGameEvent, SnapshotDto } from './game/types';

async function main() {
  // Attempt local WASM init (optional, retained for local offline dev)
  let wasmLoaded = false;
  try {
    await initGame();
    wasmLoaded = true;
  } catch (err) {
    console.warn("WASM runtime not available, operating in pure network client mode:", err);
  }

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
  const hud = new HudController();

  const timerEl = document.getElementById('timer') as HTMLElement;
  const roundEl = document.getElementById('round-val') as HTMLElement;
  const statusEl = document.getElementById('status-msg') as HTMLElement;
  const endTurnBtn = document.getElementById('end-turn-btn') as HTMLButtonElement;
  const restartBtn = document.getElementById('restart-btn') as HTMLButtonElement;
  const gameOverModal = document.getElementById('game-over-modal') as HTMLElement;
  const gameOverTitle = document.getElementById('game-over-title') as HTMLElement;
  const modalRestartBtn = document.getElementById('btn-restart-modal') as HTMLButtonElement;

  // Matchmaking UI elements
  const matchModal = document.getElementById('match-dialog-modal') as HTMLElement;
  const btnOpenMatchmaking = document.getElementById('btn-open-matchmaking') as HTMLButtonElement;
  const btnStartPvAI = document.getElementById('btn-start-pvai') as HTMLButtonElement;
  const btnStartPvP = document.getElementById('btn-start-pvp') as HTMLButtonElement;
  const btnSubmitJoin = document.getElementById('btn-submit-join') as HTMLButtonElement;
  const inputJoinId = document.getElementById('input-join-id') as HTMLInputElement;
  const btnCloseModal = document.getElementById('btn-close-modal') as HTMLButtonElement;

  if (timerEl) timer.setDisplay(timerEl);

  const session = new ClientSession();
  let isOnline = false;

  const input = new InputHandler(renderer, null);

  const renderCurrentState = () => {
    if (isOnline && session.getSnapshot()) {
      const snap = session.getSnapshot()!;
      const state = snapshotToGameState(snap);
      const hexes = snap.map.walkable;
      const obstacles = snap.map.obstacles;
      const fog = snap.visible_hexes;

      renderer.drawMap(hexes, obstacles);
      renderer.drawUnits(state);
      renderer.drawFog(fog, hexes);

      if (roundEl) roundEl.textContent = `Round ${state.round}`;

      if (state.phase === 'MatchEnd') {
        const team = session.getCurrentTeam();
        const won = state.winner === team;
        const isDraw = state.winner === null;
        let resultText = '';
        if (won) {
          resultText = 'VICTORY — BASE SECURED';
        } else if (isDraw) {
          resultText = 'DRAW — MUTUAL ANNIHILATION';
        } else {
          resultText = 'DEFEAT — STRUCTURE DESTROYED';
        }

        if (statusEl) {
          statusEl.textContent = resultText;
          statusEl.style.color = won ? '#00e676' : isDraw ? '#ffea00' : '#ff1744';
        }
        timer.stop();
        if (endTurnBtn) endTurnBtn.disabled = true;
        if (restartBtn) restartBtn.style.display = 'inline-block';

        if (gameOverModal && gameOverTitle) {
          gameOverTitle.textContent = won ? 'VICTORY' : isDraw ? 'DRAW' : 'DEFEAT';
          gameOverTitle.className = won ? 'victory' : isDraw ? '' : 'defeat';
          gameOverModal.style.display = 'flex';
        }
      }
    } else if (wasmLoaded) {
      const state = getPlayerState(0);
      const fog = getPlayerFog();
      const hexes = getMapHexes();
      const obstacles = getObstacles();

      renderer.drawMap(hexes, obstacles);
      renderer.drawUnits(state);
      renderer.drawFog(fog, hexes);

      if (roundEl) roundEl.textContent = `Round ${state.round}`;

      if (state.phase === 'MatchEnd') {
        const won = state.winner === 0;
        const isDraw = state.winner === null;
        if (statusEl) {
          statusEl.textContent = won ? 'VICTORY' : isDraw ? 'DRAW' : 'DEFEAT';
          statusEl.style.color = won ? '#00e676' : isDraw ? '#ffea00' : '#ff1744';
        }
        timer.stop();
        if (endTurnBtn) endTurnBtn.disabled = true;
        if (restartBtn) restartBtn.style.display = 'inline-block';
      } else {
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
    }
  };

  let pendingRoundStarted: { round: number; deadlineUnixMs: number; snapshot: SnapshotDto } | null = null;
  let pendingMatchEnded: { winner: number | null; snapshot: SnapshotDto } | null = null;

  const applyRoundStarted = (round: number, deadlineUnixMs: number, _snapshot: SnapshotDto) => {
    isOnline = true;
    input.setSession(session);
    renderer.setPlayerTeam(session.getCurrentTeam());
    renderCurrentState();

    if (statusEl) {
      statusEl.textContent = 'Planning Phase — Submit orders before timer expires';
      statusEl.style.color = '#00d2ff';
    }
    if (endTurnBtn) {
      endTurnBtn.disabled = false;
      endTurnBtn.textContent = 'End Turn';
    }

    timer.startWithDeadline(deadlineUnixMs, () => {
      if (!endTurnBtn.disabled) {
        endTurnBtn.click();
      }
    });
  };

  // Wire network event callbacks
  session.setEvents({
    onConnectionChange: (state) => {
      hud.setConnectionState(state);
      if (state === 'CONNECTED') {
        isOnline = true;
        input.setSession(session);
      } else if (state === 'DISCONNECTED') {
        if (!wasmLoaded) {
          if (statusEl) statusEl.textContent = 'Disconnected from server.';
        }
      }
    },
    onMatchJoined: (matchId, team, _snapshot) => {
      isOnline = true;
      input.setSession(session);
      renderer.setPlayerTeam(team);
      sessionStorage.setItem('hb_current_match', matchId);
      hud.setMatchInfo(matchId, false);
      renderCurrentState();
      if (statusEl) {
        statusEl.textContent = `Joined Match [Team ${team === 0 ? 'Blue' : 'Red'}] — Waiting for planning...`;
      }
    },
    onRoundStarted: (round, deadlineUnixMs, snapshot) => {
      isOnline = true;
      input.setSession(session);
      renderer.setPlayerTeam(session.getCurrentTeam());
      hud.setOpponentStatus(session.getIsPvAI() ? 'ai' : 'ready');

      if (input.getIsResolving()) {
        pendingRoundStarted = { round, deadlineUnixMs, snapshot };
        timer.startWithDeadline(deadlineUnixMs, () => {
          if (!endTurnBtn.disabled) {
            endTurnBtn.click();
          }
        });
      } else {
        applyRoundStarted(round, deadlineUnixMs, snapshot);
      }
    },
    onOrdersAccepted: (round) => {
      if (statusEl) {
        statusEl.textContent = `Orders locked in for Round ${round}. Awaiting opponent...`;
        statusEl.style.color = '#ffea00';
      }
    },
    onOrderRejected: (_round, _code, reason) => {
      hud.showToast(`Order rejected: ${reason}`);
      if (endTurnBtn) endTurnBtn.disabled = false;
    },
    onRoundResolved: (round, events, _snapshot) => {
      if (endTurnBtn) endTurnBtn.disabled = true;
      if (statusEl) statusEl.textContent = `Resolving Round ${round}...`;

      input.handleServerResolved(events, animator, () => {
        if (pendingMatchEnded) {
          timer.stop();
          renderCurrentState();
          pendingMatchEnded = null;
        } else if (pendingRoundStarted) {
          const { round: r, deadlineUnixMs, snapshot: snap } = pendingRoundStarted;
          pendingRoundStarted = null;
          applyRoundStarted(r, deadlineUnixMs, snap);
        } else {
          renderCurrentState();
        }
      });
    },
    onMatchEnded: (winner, snapshot) => {
      if (input.getIsResolving()) {
        pendingMatchEnded = { winner, snapshot };
      } else {
        timer.stop();
        renderCurrentState();
      }
    },
    onError: (msg) => {
      hud.showToast(`Error: ${msg}`);
    },
  });

  const handleEndTurn = () => {
    endTurnBtn.disabled = true;
    if (statusEl) statusEl.textContent = 'Orders submitted. Awaiting resolution...';

    if (isOnline) {
      session.submitOrders();
    } else {
      timer.stop();
      input.endTurnWithAnimation(animator, () => {
        renderCurrentState();
      });
    }
  };

  if (endTurnBtn) {
    endTurnBtn.addEventListener('click', handleEndTurn);
  }

  const handleRestart = () => {
    input.reset();
    sessionStorage.removeItem('hb_current_match');
    if (typeof window !== 'undefined' && window.history) {
      const url = new URL(window.location.href);
      url.searchParams.delete('match');
      url.searchParams.delete('mode');
      window.history.replaceState(null, '', url.toString());
    }
    if (isOnline) {
      startNewPvAIMatch();
    } else if (wasmLoaded) {
      restartLocalGame();
      renderCurrentState();
    }
    if (restartBtn) restartBtn.style.display = 'none';
    if (gameOverModal) gameOverModal.style.display = 'none';
  };

  if (restartBtn) restartBtn.addEventListener('click', handleRestart);
  if (modalRestartBtn) modalRestartBtn.addEventListener('click', handleRestart);

  // Matchmaking handlers
  const startNewPvAIMatch = async () => {
    try {
      if (statusEl) statusEl.textContent = 'Creating PvAI match on server...';
      const matchId = await session.createMatch(true, 30);
      if (typeof window !== 'undefined' && window.history) {
        const url = new URL(window.location.href);
        url.searchParams.set('match', matchId);
        url.searchParams.delete('mode');
        window.history.replaceState(null, '', url.toString());
      }
      sessionStorage.setItem('hb_current_match', matchId);
      hud.setMatchInfo(matchId, true);
      hud.setOpponentStatus('ai');
      session.joinMatch(matchId);
      if (matchModal) matchModal.style.display = 'none';
    } catch (err) {
      console.warn("Could not start server match, falling back to local WASM:", err);
      fallbackToLocalWasm();
    }
  };

  const startNewPvPMatch = async () => {
    try {
      if (statusEl) statusEl.textContent = 'Hosting 1v1 PvP match...';
      const matchId = await session.createMatch(false, 30);
      if (typeof window !== 'undefined' && window.history) {
        const url = new URL(window.location.href);
        url.searchParams.set('match', matchId);
        url.searchParams.delete('mode');
        window.history.replaceState(null, '', url.toString());
      }
      sessionStorage.setItem('hb_current_match', matchId);
      hud.setMatchInfo(matchId, false);
      hud.setOpponentStatus('waiting');
      session.joinMatch(matchId);
      if (matchModal) matchModal.style.display = 'none';
      hud.showToast('📋 Match created! Share link with opponent.');
    } catch (err) {
      hud.showToast('Failed to create PvP match.');
    }
  };

  const joinExistingMatch = (id: string) => {
    if (!id.trim()) return;
    const cleanId = id.trim();
    if (typeof window !== 'undefined' && window.history) {
      const url = new URL(window.location.href);
      url.searchParams.set('match', cleanId);
      url.searchParams.delete('mode');
      window.history.replaceState(null, '', url.toString());
    }
    sessionStorage.setItem('hb_current_match', cleanId);
    session.setIsPvAI(false);
    hud.setMatchInfo(cleanId, false);
    session.joinMatch(cleanId);
    if (matchModal) matchModal.style.display = 'none';
  };

  if (btnOpenMatchmaking && matchModal) {
    btnOpenMatchmaking.addEventListener('click', () => {
      matchModal.style.display = 'flex';
    });
  }
  if (btnCloseModal && matchModal) {
    btnCloseModal.addEventListener('click', () => {
      matchModal.style.display = 'none';
    });
  }
  if (btnStartPvAI) btnStartPvAI.addEventListener('click', startNewPvAIMatch);
  if (btnStartPvP) btnStartPvP.addEventListener('click', startNewPvPMatch);
  if (btnSubmitJoin && inputJoinId) {
    btnSubmitJoin.addEventListener('click', () => joinExistingMatch(inputJoinId.value));
  }

  const fallbackToLocalWasm = () => {
    if (wasmLoaded) {
      isOnline = false;
      input.setSession(null);
      hud.setConnectionState('DISCONNECTED');
      hud.setMatchInfo(null, false);
      hud.setOpponentStatus('hidden');
      renderCurrentState();
      timer.start(() => {
        if (!endTurnBtn.disabled) endTurnBtn.click();
      });
    }
  };

  // Check URL query parameters for match join or mode (e.g. ?match=123 or ?mode=pvp or ?offline=1)
  const urlParams = new URLSearchParams(window.location.search);
  const matchParam = urlParams.get('match');
  const modeParam = urlParams.get('mode');
  const offlineParam = urlParams.get('offline');
  const cachedMatch = sessionStorage.getItem('hb_current_match');

  if (matchParam) {
    joinExistingMatch(matchParam);
  } else if (modeParam === 'pvp') {
    startNewPvPMatch();
  } else if (cachedMatch && offlineParam !== '1') {
    joinExistingMatch(cachedMatch);
  } else if (offlineParam === '1' && wasmLoaded) {
    fallbackToLocalWasm();
  } else {
    // Default: Deploy into authoritative server PvAI battle (auto-falls back to WASM if server offline)
    startNewPvAIMatch();
  }

  console.log("Hexabellum Phase 3 Authoritative Multiplayer Slice Initialized.");
}

main().catch(console.error);
