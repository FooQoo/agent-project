import { expect, test, type Page } from '../helpers/fixtures';
import { agpjOk, mainProject, mainUrl, resetBoard } from '../helpers/agpj';
import { resetPreferences } from '../helpers/preferences';
import { openHeaderItem } from '../helpers/ui';

// The mode switch of the settings dialog and the sleep chip of the header. The mode is saved in
// the data area's internal folder, so the CLI sees what the board saved and the board follows a
// change the CLI makes, without a reload.

test.beforeEach(async () => {
  resetBoard(mainProject);
  await resetPreferences();
  agpjOk(mainProject, ['mode', 'sparring']);
});

test.afterEach(() => {
  agpjOk(mainProject, ['mode', 'sparring']);
});

const chip = (page: Page) => page.getByRole('banner').getByRole('status');

async function openSettings(page: Page) {
  await openHeaderItem(page, 'Settings');
  const dialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(dialog).toBeVisible();
  return dialog;
}

test('the board switches to sleep, shows the chip, and follows the CLI', async ({ page }) => {
  await page.goto(mainUrl());
  await expect(chip(page)).toHaveCount(0);

  const dialog = await openSettings(page);
  const modes = dialog.getByRole('radiogroup', { name: 'Mode' });
  await expect(modes.getByRole('radio', { name: 'Sparring' })).toBeChecked();
  // The two options are the same size.
  const sparring = await modes.getByRole('radio', { name: 'Sparring' }).boundingBox();
  const sleep = await modes.getByRole('radio', { name: 'Sleep' }).boundingBox();
  expect(Math.abs(sparring!.width - sleep!.width)).toBeLessThan(1);
  expect(Math.abs(sparring!.height - sleep!.height)).toBeLessThan(1);

  await modes.getByRole('radio', { name: 'Sleep' }).click();
  await expect(modes.getByRole('radio', { name: 'Sleep' })).toBeChecked();
  await page.keyboard.press('Escape');
  await expect(chip(page)).toBeVisible();
  // What the board saved is what the CLI reads, and a question is refused while it sleeps.
  expect(agpjOk(mainProject, ['mode']).stdout).toContain('Mode: sleep');

  await page.reload();
  await expect(chip(page)).toBeVisible();

  // A change by the CLI reaches the open page through the event stream.
  agpjOk(mainProject, ['mode', 'sparring']);
  await expect(chip(page)).toHaveCount(0, { timeout: 15_000 });
  agpjOk(mainProject, ['mode', 'sleep']);
  await expect(chip(page)).toBeVisible({ timeout: 15_000 });
});
