import { test, expect } from '@playwright/test';

test.describe('Hexabellum Tutorial Level & Mode Selection FTUE', () => {
  test('verifies mode selection hub, choosing tutorial vs standard game, and switching modes', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // 1. Visit root page
    await page.goto('http://localhost:5173/');
    await expect(page).toHaveTitle('Hexabellum — Tactical Hex MOBA');

    // 2. Open Mode Selection Hub
    const btnOpenModeSelect = page.locator('#btn-open-mode-select');
    await expect(btnOpenModeSelect).toBeVisible();
    await btnOpenModeSelect.click();

    const modeModal = page.locator('#mode-selection-modal');
    await expect(modeModal).toBeVisible();

    // Verify both featured cards are present and styled
    await expect(modeModal.locator('.mode-card-tutorial')).toBeVisible();
    await expect(modeModal.locator('.mode-card-arena')).toBeVisible();
    await expect(modeModal.locator('.mode-hub-title')).toHaveText('HEXABELLUM — PORTAL DA CONVERGÊNCIA');

    // 3. Launch Archmage Trial Tutorial
    const btnStartTutorial = page.locator('#btn-hub-start-tutorial');
    await btnStartTutorial.click();
    await expect(modeModal).not.toBeVisible();

    // Verify Archmage dialogue card is rendered
    const wizardCard = page.locator('.wizard-card');
    await expect(wizardCard).toBeVisible({ timeout: 5000 });
    await expect(page.locator('.wizard-name')).toHaveText('Archmage of Convergence');

    // Verify Objective checklist dock is pinned
    const dock = page.locator('#tutorial-objective-dock');
    await expect(dock).toBeVisible();
    await expect(page.locator('.objective-title')).toHaveText('The Convergence Calls');

    // 4. Advance dialogue using Continue button
    const btnContinue = page.locator('.btn-wizard-primary');
    await btnContinue.click();

    // Verify next line / step
    await expect(page.locator('.objective-badge')).toHaveText('Step 2/2');

    // 5. Exit Tutorial and return to Standard Game
    const btnToggleTutorial = page.locator('#btn-toggle-tutorial');
    await expect(btnToggleTutorial).toHaveText('⚔️ Exit Tutorial');
    await btnToggleTutorial.click();

    // Verify standard game HUD is restored
    await expect(btnToggleTutorial).toHaveText('🎓 Tutorial');
    await expect(page.locator('#end-turn-btn')).toBeVisible();

    expect(errors).toEqual([]);
  });

  test('verifies Lesson 1 hex movement, objective ring, and collision soft-fail', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // Directly launch into Lesson 1
    await page.goto('http://localhost:5173/?mode=tutorial&lesson=lesson_01_movement');

    // 1. Dialogue Step 1: dismiss intro
    const wizardCard = page.locator('.wizard-card');
    await expect(wizardCard).toBeVisible({ timeout: 5000 });
    const btnContinue = page.locator('.btn-wizard-primary');
    await btnContinue.click();

    // 2. Objective Step 2: Move 1 hex East to (1, 0)
    const taskText = page.locator('.task-text');
    await expect(taskText).toHaveText('Move 1 hex East to coordinate (1, 0)');

    const canvas = page.locator('#game-canvas');
    await expect(canvas).toBeVisible();

    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    const cx = box!.width / 2;
    const cy = box!.height / 2;

    // Coordinate (1, 0): x = cx + 30 * sqrt(3) * 1 = cx + 51.96, y = cy
    const targetX = cx + 51.96;
    const targetY = cy;

    await page.mouse.click(targetX, targetY);

    // 3. Step 3: Collision test dialogue
    await expect(page.locator('.objective-badge')).toHaveText('Step 3/4', { timeout: 5000 });
    await expect(wizardCard).toBeVisible();
    await btnContinue.click();

    // 4. Step 4: Attempt collision into dummy at (0, 2)
    await expect(page.locator('.objective-badge')).toHaveText('Step 4/4', { timeout: 5000 });
    // q = 0, r = 2: x = cx + 30 * (sqrt(3)/2 * 2) = cx + 51.96, y = cy + 30 * 1.5 * 2 = cy + 90
    const dummyX = cx + 51.96;
    const dummyY = cy + 90;

    await page.mouse.click(dummyX, dummyY);

    // Soft-fail explanation should appear with Archmage lore
    await expect(wizardCard).toHaveClass(/card-soft-fail/, { timeout: 5000 });
    await expect(page.locator('.wizard-text')).toContainText('stone resists');

    expect(errors).toEqual([]);
  });

  test('verifies Lesson 3 initiative and death-before-acting combat resolution', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto('http://localhost:5173/?mode=tutorial&lesson=lesson_03_initiative');

    // 1. Dismiss briefing
    const btnContinue = page.locator('.btn-wizard-primary');
    await btnContinue.click();
    await expect(page.locator('.objective-badge')).toHaveText('Step 2/3', { timeout: 5000 });

    // 2. Attack dummy at (0, 0)
    const canvas = page.locator('#game-canvas');
    const box = await canvas.boundingBox();
    const cx = box!.width / 2;
    const cy = box!.height / 2;

    // Click dummy at (0, 0)
    await page.mouse.click(cx, cy);

    // 3. Resolution debrief dialogue: dummy died at initiative 4 before acting at initiative 2
    const wizardCard = page.locator('.wizard-card');
    await expect(wizardCard).toBeVisible({ timeout: 10000 });
    await expect(page.locator('.wizard-text')).toContainText('Death-Before-Acting');

    expect(errors).toEqual([]);
  });
});
