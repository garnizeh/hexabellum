import * as PIXI from 'pixi.js';
import './ui/tutorial.css';
import './ui/phase5.css';
import './ui/phase6.css';
import './ui/phase7.css';
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
import { TutorialController } from './game/tutorial/TutorialController';
import { ModeSelectModal } from './ui/ModeSelectModal';
import { AbilityDock } from './ui/AbilityDock';
import { LobbyScreen } from './ui/LobbyScreen';
import { HeroSelectScreen } from './ui/HeroSelectScreen';
import { ShopDrawer } from './ui/ShopDrawer';
import { TacticalMinimap } from './ui/Minimap';

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
    preference: 'webgl',
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
  const btnStart5v5 = document.getElementById('btn-start-5v5') as HTMLButtonElement;
  const btnStartPvAI = document.getElementById('btn-start-pvai') as HTMLButtonElement;
  const btnStartPvP = document.getElementById('btn-start-pvp') as HTMLButtonElement;
  const btnSubmitJoin = document.getElementById('btn-submit-join') as HTMLButtonElement;
  const inputJoinId = document.getElementById('input-join-id') as HTMLInputElement;
  const btnCloseModal = document.getElementById('btn-close-modal') as HTMLButtonElement;

  if (timerEl) timer.setDisplay(timerEl);

  const session = new ClientSession();
  let isOnline = false;

  const lobbyScreen = new LobbyScreen();
  lobbyScreen.setSession(session);

  const heroSelectScreen = new HeroSelectScreen();
  heroSelectScreen.setSession(session);

  const input = new InputHandler(renderer, null);
  const abilityDock = new AbilityDock();
  abilityDock.setInputHandler(input);
  abilityDock.setSession(session);

  const shopDrawer = new ShopDrawer();
  shopDrawer.setSession(session);

  const minimap = new TacticalMinimap();
  minimap.setRenderer(renderer);
  minimap.setOnPing((ping) => {
    hud.showToast(`Tactical Ping: ${ping.label}`);
  });

  input.setOnSkipResolution(() => {
    animator.skipToEnd();
  });

  app.ticker.add(() => {
    minimap.render();
  });

  abilityDock.setOnOpenShop(() => {
    shopDrawer.toggle();
  });

  input.setOnUnitSelectedChange((unit) => {
    abilityDock.update(unit);
  });

  // Provide hero position for Space key snap
  renderer.getCamera().setHeroPositionProvider(() => {
    const myHero = session.getMyHero();
    if (myHero) {
      return renderer.hexToPixel(myHero.pos.q, myHero.pos.r);
    }
    const selected = input.getSelectedUnit();
    if (selected) {
      return renderer.hexToPixel(selected.pos.q, selected.pos.r);
    }
    return null;
  });

  abilityDock.setOnCenterHero(() => {
    renderer.getCamera().centerOnHero();
  });

  hud.setOnFocusUnit((unitId) => {
    const snap = session.getSnapshot();
    const unit = snap?.units.find((u) => u.id === unitId);
    if (unit) {
      renderer.centerOnHex(unit.pos.q, unit.pos.r);
    }
  });

  const tutorial = new TutorialController(app, renderer, animator);
  const modeModal = new ModeSelectModal();

  input.setClickInterceptor((x, y) => {
    if (tutorial.isRunning()) {
      return tutorial.handleCanvasClick(x, y);
    }
    return false;
  });

  const renderCurrentState = () => {
    if (tutorial.isRunning()) {
      return;
    }
    if (isOnline && session.getSnapshot()) {
      const snap = session.getSnapshot()!;
      const state = snapshotToGameState(snap);
      const hexes = snap.map.walkable;
      const obstacles = snap.map.obstacles;
      const fog = snap.visible_hexes;

      renderer.drawMap(hexes, obstacles);
      if (snap.base_zones) {
        renderer.drawBaseZones(snap.base_zones);
      }
      renderer.drawUnits(state);
      renderer.drawFog(fog, hexes);
      minimap.update(snap);

      // Auto-select player's assigned hero in 5v5 if none selected
      const primaryId = session.getPrimaryControlledUnitId();
      if (
        primaryId &&
        input.getSelectedUnitId() === null &&
        state.units[primaryId] &&
        state.units[primaryId].hp > 0 &&
        state.units[primaryId].life_state !== 'dead_awaiting_respawn'
      ) {
        input.selectUnit(primaryId);
      }

      abilityDock.update(input.getSelectedUnit());
      hud.update5v5Rosters(snap, session.getNet().getPlayerId());
      hud.updateMacroBar(snap);

      // Hero defeated respawn banner check
      const isDead = session.isHeroDead();
      if (isDead) {
        const rounds = session.getHeroRespawnRounds() ?? 3;
        hud.setHeroDefeated(true, rounds);
      } else {
        hud.setHeroDefeated(false, 0);
      }

      if (roundEl) roundEl.textContent = `Round ${state.round}`;

      if (state.phase === 'MatchEnd') {
        const team = session.getCurrentTeam();
        const won = state.winner === team;
        const isDraw = state.winner === null;
        let resultText = '';
        if (won) {
          resultText = 'VICTORY — ENEMY CORE OBLITERATED';
        } else if (isDraw) {
          resultText = 'DRAW — MUTUAL CORE DESTRUCTION';
        } else {
          resultText = 'DEFEAT — ALLIED CORE COLLAPSED';
        }

        if (statusEl) {
          statusEl.textContent = resultText;
          statusEl.style.color = won ? '#00e676' : isDraw ? '#ffea00' : '#ff1744';
        }
        timer.stop();
        if (endTurnBtn) endTurnBtn.disabled = true;
        if (restartBtn) restartBtn.style.display = 'inline-block';

        const myHero = session.getMyHero();
        const controlledEco = session.getControlledHeroEconomy();
        const coreHp = snap.core_hp;
        const myCoreHp = coreHp ? coreHp[team]?.[0] ?? 0 : 0;
        const enemyTeam = team === 0 ? 1 : 0;
        const enemyCoreHp = coreHp ? coreHp[enemyTeam]?.[0] ?? 0 : 0;
        const vaultSecured = snap.objective ? !snap.objective.is_alive : false;

        hud.showVictoryCelebrationModal({
          won,
          isDraw,
          round: state.round,
          stats: {
            alliedCoreHp: myCoreHp,
            enemyCoreHp,
            vaultSecured,
            totalGold: controlledEco?.gold ?? myHero?.gold ?? 50,
            items: controlledEco?.items ?? myHero?.items ?? [],
          },
          onReturnToLobby: () => {
            window.location.reload();
          },
        });
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

  let pendingRoundStarted: {
    round: number;
    deadlineUnixMs: number;
    snapshot: SnapshotDto;
    events?: SanitizedGameEvent[];
  } | null = null;
  let pendingMatchEnded: { winner: number | null; snapshot: SnapshotDto } | null = null;

  const applyRoundStarted = (
    round: number,
    deadlineUnixMs: number,
    _snapshot: SnapshotDto,
    events?: SanitizedGameEvent[]
  ) => {
    isOnline = true;
    lobbyScreen.hide();
    heroSelectScreen.hide();
    input.setSession(session);
    abilityDock.setSession(session);
    shopDrawer.setSession(session);
    shopDrawer.update();
    renderer.setPlayerTeam(session.getCurrentTeam());
    renderCurrentState();

    if (events && events.length > 0) {
      animator.playEvents(events);
    }

    if (statusEl) {
      statusEl.textContent = 'Planning Phase — Submit orders before timer expires';
      statusEl.style.color = '#00d2ff';
    }
    if (endTurnBtn) {
      endTurnBtn.disabled = false;
      endTurnBtn.textContent = 'End Turn';
    }

    timer.startWithDeadline(deadlineUnixMs, () => {
      if (endTurnBtn) endTurnBtn.disabled = true;
      if (statusEl) {
        statusEl.textContent = 'Turn deadline reached — resolving with server...';
        statusEl.style.color = '#ffea00';
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
    onLobbyUpdated: (lobby) => {
      lobbyScreen.update(lobby);
      heroSelectScreen.update(lobby);
    },
    onHeroSelected: () => {
      const lobby = session.getLobbyState();
      if (lobby) {
        heroSelectScreen.update(lobby);
      }
    },
    onMatchStarting: () => {
      lobbyScreen.hide();
      heroSelectScreen.hide();
      if (statusEl) {
        statusEl.textContent = 'Hero draft complete! Match deploying into arena...';
        statusEl.style.color = '#00e676';
      }
    },
    onEarlyResolutionTriggered: (round) => {
      if (statusEl) {
        statusEl.textContent = `All orders locked in! Early resolution triggered for Round ${round}...`;
        statusEl.style.color = '#00e676';
      }
    },
    onPlayerConnectionUpdated: (playerId, connected, isAi) => {
      if (isAi) {
        hud.showToast(`Player ${playerId.slice(0, 6)} disconnected — AI takeover activated.`);
      } else if (connected) {
        hud.showToast(`Player ${playerId.slice(0, 6)} reconnected! Control restored.`);
      }
    },
    onRoundStarted: (round, deadlineUnixMs, snapshot, events) => {
      isOnline = true;
      lobbyScreen.hide();
      heroSelectScreen.hide();
      input.setSession(session);
      renderer.setPlayerTeam(session.getCurrentTeam());
      hud.setOpponentStatus(session.getIsPvAI() ? 'ai' : 'ready');

      if (input.getIsResolving()) {
        pendingRoundStarted = { round, deadlineUnixMs, snapshot, events };
        timer.startWithDeadline(deadlineUnixMs, () => {
          if (endTurnBtn) endTurnBtn.disabled = true;
          if (statusEl) {
            statusEl.textContent = 'Turn deadline reached — resolving with server...';
            statusEl.style.color = '#ffea00';
          }
        });
      } else {
        applyRoundStarted(round, deadlineUnixMs, snapshot, events);
      }
    },
    onPurchaseResolved: (_unitId, itemId, success, _goldRemaining, error) => {
      shopDrawer.update();
      abilityDock.update(input.getSelectedUnit());
      renderCurrentState();
      if (success) {
        hud.showToast(`✅ Purchased ${itemId.toUpperCase()}!`);
      } else {
        hud.showToast(`❌ Purchase failed: ${error ?? 'Invalid request'}`);
      }
    },
    onEconomyUpdated: (unitId, _gold, _xp, _level, items) => {
      shopDrawer.update();
      abilityDock.update(input.getSelectedUnit());
      renderCurrentState();
      const myId = session.getPrimaryControlledUnitId();
      if (myId !== null && unitId !== myId) {
        const itemBought = items[items.length - 1];
        if (itemBought) {
          hud.showToast(`🛡️ Teammate #${unitId} bought ${itemBought.toUpperCase()}`);
        }
      }
    },
    onLevelUpOccurred: (_unitId, newLevel) => {
      abilityDock.update(input.getSelectedUnit());
      renderCurrentState();
      hud.showToast(`⭐ LEVEL UP! Now Level ${newLevel}`);
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
      console.debug(
        `[Hexabellum] Round ${round} resolved. Server state hash: ${_snapshot.state_hash}`
      );

      for (const ev of events) {
        if (ev.type === 'RewardGranted') {
          if (ev.reason === 'HeroKill') {
            hud.showToast(`⚔️ Hero eliminated! +${ev.gold}G, +${ev.xp}XP`);
          } else if (ev.reason === 'TowerDestroyed') {
            hud.showToast(`🏰 Tower destroyed! Team bounty +${ev.gold}G, +${ev.xp}XP`);
          } else if (ev.reason === 'SpawnerDestroyed') {
            hud.showToast(`⚡ Spawner destroyed! Team bounty +${ev.gold}G, +${ev.xp}XP`);
          } else if (ev.reason === 'NeutralKill') {
            hud.showToast(`🐺 Neutral Guardian slain! +${ev.gold}G, +${ev.xp}XP`);
          }
        } else if (ev.type === 'LevelUp') {
          hud.showToast(`⭐ LEVEL UP! Unit #${ev.unit_id} reached Level ${ev.new_level}!`);
        } else if (ev.type === 'HeroDied') {
          hud.showToast(`💀 Allied Hero fallen! Respawning at base in ${ev.respawn_rounds} rounds.`);
        } else if (ev.type === 'HeroRespawned') {
          hud.showToast(`✨ Hero respawned at Allied Base Sanctuary!`);
        } else if (ev.type === 'ObjectiveDestroyed') {
          hud.showToast(`🏛️ ANCIENT VAULT SECURED! Team +50G, +40XP & +5 Attack Damage buff granted!`);
        } else if (ev.type === 'CoreDestroyed') {
          hud.showToast(`💥 SOVEREIGN CORE OBLITERATED!`);
        }
      }

      // Show resolution playback controls (1.0x/2.0x toggle and Skip button - docs/ui-ux.md §4.14, §8.2)
      const updatePlaybackUI = () => {
        abilityDock.showPlaybackControls(
          animator.getPlaybackSpeed(),
          () => {
            const nextSpeed = animator.getPlaybackSpeed() === 1.0 ? 2.0 : 1.0;
            animator.setPlaybackSpeed(nextSpeed);
            updatePlaybackUI();
          },
          () => {
            animator.skipToEnd();
          }
        );
      };
      updatePlaybackUI();

      input.handleServerResolved(events, animator, () => {
        abilityDock.hidePlaybackControls();
        if (pendingMatchEnded) {
          timer.stop();
          renderCurrentState();
          pendingMatchEnded = null;
        } else if (pendingRoundStarted) {
          const { round: r, deadlineUnixMs, snapshot: snap, events: evs } = pendingRoundStarted;
          pendingRoundStarted = null;
          applyRoundStarted(r, deadlineUnixMs, snap, evs);
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
    onOpponentStatus: (online) => {
      if (!session.getIsPvAI()) {
        hud.setOpponentStatus(online ? 'ready' : 'offline');
      }
    },
    onError: (msg) => {
      hud.showToast(`Error: ${msg}`);
    },
  });

  const handleEndTurn = () => {
    endTurnBtn.disabled = true;

    if (isOnline) {
      if (session.getStagedCount() === 0) {
        session.stageWaitOrdersForControlled();
      }
      if (statusEl) {
        statusEl.textContent = 'Orders submitted. Awaiting resolution...';
        statusEl.style.color = '#00d2ff';
      }
      session.submitOrders();
    } else {
      if (statusEl) statusEl.textContent = 'Orders submitted. Awaiting resolution...';
      timer.stop();
      const updatePlaybackUI = () => {
        abilityDock.showPlaybackControls(
          animator.getPlaybackSpeed(),
          () => {
            const nextSpeed = animator.getPlaybackSpeed() === 1.0 ? 2.0 : 1.0;
            animator.setPlaybackSpeed(nextSpeed);
            updatePlaybackUI();
          },
          () => {
            animator.skipToEnd();
          }
        );
      };
      updatePlaybackUI();

      input.endTurnWithAnimation(animator, () => {
        abilityDock.hidePlaybackControls();
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
      console.info("Multiplayer server offline; seamlessly running match via local WASM engine:", err);
      hud.showToast('Backend server offline. Running in local WASM mode.');
      fallbackToLocalWasm();
    }
  };

  const startNew5v5Match = async () => {
    try {
      if (statusEl) statusEl.textContent = 'Hosting 5v5 Tactical Match (10 Players)...';
      const matchId = await session.create5v5Match(true, 30);
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
      hud.showToast('⚔️ 5v5 Match created! Entering pre-game lobby.');
    } catch (err) {
      console.warn("Failed to create 5v5 match:", err);
      hud.showToast('Failed to create 5v5 match. Ensure backend server is running.');
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
      console.warn("Failed to create PvP match:", err);
      hud.showToast('Failed to create PvP match. Ensure the backend server is running.');
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

  const btnOpenModeSelect = document.getElementById('btn-open-mode-select') as HTMLButtonElement;
  const btnToggleTutorial = document.getElementById('btn-toggle-tutorial') as HTMLButtonElement;

  const startTutorialMode = (lessonId: string = 'lesson_00_intro') => {
    timer.stop();
    modeModal.hideFtueBanner();

    if (endTurnBtn) endTurnBtn.style.display = 'none';
    if (restartBtn) restartBtn.style.display = 'none';
    if (gameOverModal) gameOverModal.style.display = 'none';

    // Hide overlapping arena UI elements during tutorial
    const legendEl = document.getElementById('legend');
    if (legendEl) legendEl.style.display = 'none';
    const connPillEl = document.getElementById('conn-pill');
    if (connPillEl) connPillEl.style.display = 'none';
    const matchIdEl = document.getElementById('match-id-display');
    if (matchIdEl) matchIdEl.style.display = 'none';
    const copyBtnEl = document.getElementById('copy-match-btn');
    if (copyBtnEl) copyBtnEl.style.display = 'none';
    const matchMenuBtn = document.getElementById('btn-open-matchmaking');
    if (matchMenuBtn) matchMenuBtn.style.display = 'none';
    const oppStatusEl = document.getElementById('opponent-status');
    if (oppStatusEl) oppStatusEl.style.display = 'none';

    if (timerEl) {
      timerEl.textContent = '∞';
      timerEl.title = 'Timer Paused — No time limit in Archmage Trial';
    }

    if (btnToggleTutorial) {
      btnToggleTutorial.textContent = '⚔️ Exit Tutorial';
      btnToggleTutorial.style.color = '#ffea00';
      btnToggleTutorial.style.borderColor = 'rgba(255, 234, 0, 0.5)';
    }
    if (statusEl) {
      statusEl.textContent = '🎓 Archmage Trial Mode — Follow objective guidance';
      statusEl.style.color = '#00e676';
    }
    if (roundEl) {
      roundEl.textContent = 'Tutorial';
    }
    tutorial.startLesson(lessonId);
  };

  const exitTutorialMode = () => {
    // Restore standard arena UI elements
    const legendEl = document.getElementById('legend');
    if (legendEl) legendEl.style.display = 'block';
    const connPillEl = document.getElementById('conn-pill');
    if (connPillEl) connPillEl.style.display = 'inline-flex';
    const matchMenuBtn = document.getElementById('btn-open-matchmaking');
    if (matchMenuBtn) matchMenuBtn.style.display = 'inline-block';
    if (isOnline) {
      const matchIdEl = document.getElementById('match-id-display');
      if (matchIdEl) matchIdEl.style.display = 'inline';
    }

    if (btnToggleTutorial) {
      btnToggleTutorial.textContent = '🎓 Tutorial';
      btnToggleTutorial.style.color = '#00e676';
      btnToggleTutorial.style.borderColor = 'rgba(0, 230, 118, 0.4)';
    }
    if (endTurnBtn) endTurnBtn.style.display = 'inline-block';
    renderCurrentState();
  };

  tutorial.setOnExit(() => {
    exitTutorialMode();
  });

  if (btnOpenModeSelect) {
    btnOpenModeSelect.addEventListener('click', () => {
      modeModal.open();
    });
  }

  if (btnToggleTutorial) {
    btnToggleTutorial.addEventListener('click', () => {
      if (tutorial.isRunning()) {
        tutorial.exitTutorial();
      } else {
        modeModal.open();
      }
    });
  }

  modeModal.setOnStartTutorial((lessonId) => {
    startTutorialMode(lessonId);
  });

  modeModal.setOnStart5v5(() => {
    if (tutorial.isRunning()) {
      tutorial.exitTutorial();
    }
    startNew5v5Match();
  });

  modeModal.setOnStartPvAI(() => {
    if (tutorial.isRunning()) {
      tutorial.exitTutorial();
    }
    startNewPvAIMatch();
  });

  modeModal.setOnStartPvP(() => {
    if (tutorial.isRunning()) {
      tutorial.exitTutorial();
    }
    startNewPvPMatch();
  });

  modeModal.setOnJoinMatch((matchId) => {
    if (tutorial.isRunning()) {
      tutorial.exitTutorial();
    }
    joinExistingMatch(matchId);
  });

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
  if (btnStart5v5) btnStart5v5.addEventListener('click', startNew5v5Match);
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

  // Check URL query parameters for match join or mode (e.g. ?match=123 or ?mode=pvp or ?mode=tutorial or ?offline=1)
  const urlParams = new URLSearchParams(window.location.search);
  const matchParam = urlParams.get('match');
  const modeParam = urlParams.get('mode');
  const lessonParam = urlParams.get('lesson');
  const offlineParam = urlParams.get('offline');
  const cachedMatch = sessionStorage.getItem('hb_current_match');

  // FTUE Prompt: for fresh users on desktop/web, offer the Archmage's Trial
  try {
    const tutStorage = localStorage.getItem('hexabellum_tutorial_v1');
    const bannerDismissed = localStorage.getItem('hexabellum_ftue_banner_dismissed');
    if (!tutStorage && !bannerDismissed && modeParam !== 'tutorial' && !matchParam) {
      modeModal.showFtueBanner(() => {
        startTutorialMode('lesson_00_intro');
      });
    }
  } catch {}

  if (modeParam === 'tutorial') {
    startTutorialMode(lessonParam || 'lesson_00_intro');
  } else if (matchParam) {
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

  console.log("Hexabellum Tactical Engine & Tutorial Subsystems Ready.");
}

main().catch(console.error);
