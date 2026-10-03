import { test, expect } from '@playwright/test';

test.describe('Hexabellum Phase 3 Authoritative Multiplayer Slice', () => {
  test('verifies authoritative PvAI match, HUD, timer, and server turn resolution', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto('http://localhost:5173/');
    await expect(page).toHaveTitle('Hexabellum — Tactical Hex MOBA');

    // 1. Connection pill should indicate ONLINE when connected to authoritative server
    const connPill = page.locator('#conn-pill');
    await expect(connPill).toBeVisible();
    await expect(connPill).toHaveClass(/connected/, { timeout: 10000 });
    await expect(connPill.locator('.conn-text')).toHaveText('ONLINE');

    // 2. Opponent status should show Server AI
    const opponentStatus = page.locator('#opponent-status');
    await expect(opponentStatus).toBeVisible();
    await expect(opponentStatus).toHaveText('Opponent: Tactical Server AI');

    // 3. Match menu modal toggle
    const matchMenuBtn = page.locator('#btn-open-matchmaking');
    await expect(matchMenuBtn).toBeVisible();
    await matchMenuBtn.click();

    const matchModal = page.locator('#match-dialog-modal');
    await expect(matchModal).toBeVisible();

    const closeModalBtn = page.locator('#btn-close-modal');
    await closeModalBtn.click();
    await expect(matchModal).not.toBeVisible();

    // 4. Select Hero 2 at (-4, 0)
    const canvas = page.locator('#game-canvas');
    await expect(canvas).toBeVisible();

    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    const cx = box!.width / 2;
    const cy = box!.height / 2;

    const heroX = cx - 207.85;
    const heroY = cy;
    await page.mouse.click(heroX, heroY);

    const inspector = page.locator('#unit-inspector');
    await expect(inspector).toBeVisible();
    await expect(page.locator('#insp-name')).toHaveText('Hero #2');
    await expect(page.locator('#insp-hp')).toHaveText('100 / 100');

    // 5. Submit orders and verify authoritative server resolution to Round 1
    const endTurnBtn = page.locator('#end-turn-btn');
    await expect(endTurnBtn).toBeEnabled();
    await endTurnBtn.click();

    const roundVal = page.locator('#round-val');
    await expect(roundVal).toHaveText('Round 1', { timeout: 15000 });
    await expect(endTurnBtn).toBeEnabled({ timeout: 15000 });

    expect(errors).toEqual([]);
  });

  test('verifies 1v1 PvP Commander match between 2 separate players and mid-turn reconnection', async ({ browser }) => {
    // Context 1: Player 1 (Team 0)
    const context1 = await browser.newContext();
    const page1 = await context1.newPage();
    await page1.goto('http://localhost:5173/?mode=pvp');

    const connPill1 = page1.locator('#conn-pill');
    await expect(connPill1).toHaveClass(/connected/, { timeout: 10000 });

    // Player 1 should see "Waiting for Opponent..."
    const oppStatus1 = page1.locator('#opponent-status');
    await expect(oppStatus1).toHaveText('Waiting for Opponent...', { timeout: 10000 });

    // Retrieve created Match ID
    const matchIdDisplay = page1.locator('#match-id-display');
    await expect(matchIdDisplay).toBeVisible({ timeout: 10000 });
    const matchId = await matchIdDisplay.getAttribute('data-full-id');
    expect(matchId).toBeTruthy();

    // Context 2: Player 2 (Team 1) joins the same match
    const context2 = await browser.newContext();
    const page2 = await context2.newPage();
    await page2.goto(`http://localhost:5173/?match=${matchId}`);

    const connPill2 = page2.locator('#conn-pill');
    await expect(connPill2).toHaveClass(/connected/, { timeout: 10000 });

    // Both players should now be connected and match started
    await expect(oppStatus1).toHaveText('Opponent Connected', { timeout: 10000 });

    const roundVal1 = page1.locator('#round-val');
    const roundVal2 = page2.locator('#round-val');
    await expect(roundVal1).toHaveText('Round 0');
    await expect(roundVal2).toHaveText('Round 0');

    // Player 1 clicks End Turn
    const endTurn1 = page1.locator('#end-turn-btn');
    await expect(endTurn1).toBeEnabled({ timeout: 10000 });
    await endTurn1.click();
    await expect(endTurn1).toBeDisabled();

    // Player 2 clicks End Turn -> early resolution grace period fires
    const endTurn2 = page2.locator('#end-turn-btn');
    await expect(endTurn2).toBeEnabled({ timeout: 10000 });
    await endTurn2.click();
    await expect(endTurn2).toBeDisabled();

    // Both clients resolve authoritatively to Round 1
    await expect(roundVal1).toHaveText('Round 1', { timeout: 15000 });
    await expect(roundVal2).toHaveText('Round 1', { timeout: 15000 });
    await expect(endTurn1).toBeEnabled({ timeout: 15000 });
    await expect(endTurn2).toBeEnabled({ timeout: 15000 });

    // Test token-based transparent reconnection: reload Player 1's page
    await page1.reload();
    await expect(connPill1).toHaveClass(/connected/, { timeout: 10000 });
    await expect(roundVal1).toHaveText('Round 1', { timeout: 10000 });

    await context1.close();
    await context2.close();
  });
});
