import { test, expect } from '@playwright/test';

test.describe('Hexabellum Phase 2 MOBA E2E', () => {
  test('verifies unit selection, inspector panel, path preview, and turn resolution', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('http://localhost:5173/');
    await expect(page).toHaveTitle('Hexabellum — Tactical Hex MOBA');

    const canvas = page.locator('#game-canvas');
    await expect(canvas).toBeVisible();

    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    const cx = box!.width / 2;
    const cy = box!.height / 2;

    // Hero 2 at (-4, 0): x = cx + 30 * sqrt(3) * (-4) = cx - 207.85, y = cy
    const heroX = cx - 207.85;
    const heroY = cy;

    // Click on Hero 2
    await page.mouse.click(heroX, heroY);

    // Inspector should now be visible with Hero #2 stats
    const inspector = page.locator('#unit-inspector');
    await expect(inspector).toBeVisible();
    await expect(page.locator('#insp-name')).toHaveText('Hero #2');
    await expect(page.locator('#insp-team')).toHaveText('Blue (Player)');
    await expect(page.locator('#insp-hp')).toHaveText('100 / 100');
    await expect(page.locator('#insp-ap')).toHaveText('3 / 3');

    // Screenshot selection ring & reachable move targets
    await page.screenshot({ path: 'tests/screenshot_selected_hero.png' });

    // Click on move target (-3, -1):
    // q = -3, r = -1: x = cx + 30 * (sqrt(3)*(-3) + sqrt(3)/2*(-1)) = cx + 30 * (-3*1.732 - 0.866) = cx - 181.86
    // y = cy + 30 * (1.5 * -1) = cy - 45
    const targetX = cx - 181.86;
    const targetY = cy - 45;
    await page.mouse.click(targetX, targetY);

    // Screenshot path preview
    await page.screenshot({ path: 'tests/screenshot_path_preview.png' });

    // Click End Turn (Hero 2 should move along path to (-3, -1))
    const endTurnBtn = page.locator('#end-turn-btn');
    await endTurnBtn.click();

    const roundVal = page.locator('#round-val');
    await expect(roundVal).toHaveText('Round 1', { timeout: 10000 });
    await expect(endTurnBtn).toBeEnabled({ timeout: 10000 });

    // Advance to Round 2 & Round 3
    await endTurnBtn.click();
    await expect(roundVal).toHaveText('Round 2', { timeout: 10000 });
    await expect(endTurnBtn).toBeEnabled({ timeout: 10000 });

    await endTurnBtn.click();
    await expect(roundVal).toHaveText('Round 3', { timeout: 15000 });
    await expect(endTurnBtn).toBeEnabled({ timeout: 15000 });

    await page.screenshot({ path: 'tests/screenshot_round3_minions.png' });

    expect(errors).toEqual([]);
  });
});
