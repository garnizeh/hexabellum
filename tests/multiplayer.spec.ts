import { test, expect } from '@playwright/test';

test.describe('Hexabellum Phase 3 Multiplayer HUD & Controls', () => {
  test('verifies connection pill, match menu modal, hero selection, and planning timer', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('http://localhost:5173/');
    await expect(page).toHaveTitle('Hexabellum — Tactical Hex MOBA');

    // 1. Connection pill should be visible
    const connPill = page.locator('#conn-pill');
    await expect(connPill).toBeVisible();

    // 2. Open Match Menu
    const matchMenuBtn = page.locator('#btn-open-matchmaking');
    await expect(matchMenuBtn).toBeVisible();
    await matchMenuBtn.click();

    const matchModal = page.locator('#match-dialog-modal');
    await expect(matchModal).toBeVisible();

    // Close Match Menu
    const closeModalBtn = page.locator('#btn-close-modal');
    await closeModalBtn.click();
    await expect(matchModal).not.toBeVisible();

    // 3. Game canvas is visible
    const canvas = page.locator('#game-canvas');
    await expect(canvas).toBeVisible();

    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    const cx = box!.width / 2;
    const cy = box!.height / 2;

    // Click on Hero 2 at (-4, 0)
    const heroX = cx - 207.85;
    const heroY = cy;
    await page.mouse.click(heroX, heroY);

    // Inspector should now be visible with Hero #2 stats
    const inspector = page.locator('#unit-inspector');
    await expect(inspector).toBeVisible();
    await expect(page.locator('#insp-name')).toHaveText('Hero #2');
    await expect(page.locator('#insp-hp')).toHaveText('100 / 100');

    // 4. Timer is running
    const timer = page.locator('#timer');
    await expect(timer).toBeVisible();

    // 5. Click End Turn
    const endTurnBtn = page.locator('#end-turn-btn');
    await expect(endTurnBtn).toBeEnabled();
    await endTurnBtn.click();

    const roundVal = page.locator('#round-val');
    await expect(roundVal).toHaveText('Round 1', { timeout: 10000 });
  });
});
