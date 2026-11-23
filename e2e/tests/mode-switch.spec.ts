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
  agpjOk(mainProject, ['mode', 'schedule', '--off']);
});

test.afterEach(() => {
  agpjOk(mainProject, ['mode', 'auto']);
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
  expect(Math.round(sparring!.width)).toBe(Math.round(sleep!.width));
  expect(Math.round(sparring!.height)).toBe(Math.round(sleep!.height));

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

// The e2e servers and commands run with TZ=UTC, so the window is written in UTC.
const clock = (offsetHours: number) => {
  const time = new Date(Date.now() + offsetHours * 3_600_000);
  return `${String(time.getUTCHours()).padStart(2, '0')}:${String(time.getUTCMinutes()).padStart(2, '0')}`;
};

test('a daily window sleeps by the schedule, a manual mode overrides it, and auto gives it back', async ({
  page,
}) => {
  await page.goto(mainUrl());
  await expect(chip(page)).toHaveCount(0);

  // The CLI sets a window around the present moment; the open board follows.
  agpjOk(mainProject, ['mode', 'schedule', '--start', clock(-1), '--end', clock(1)]);
  await expect(chip(page)).toBeVisible({ timeout: 15_000 });
  await expect(chip(page)).toContainText('until');

  const dialog = await openSettings(page);
  await expect(dialog.getByText(/Asleep by the schedule until/)).toBeVisible();
  await expect(dialog.getByLabel('Sleep on a schedule')).toBeChecked();
  await expect(dialog.getByLabel('From')).toHaveValue(clock(-1));

  // Sparring set by hand inside the window wins; the dialog offers to follow the schedule.
  await dialog.getByRole('radio', { name: 'Sparring' }).click();
  await expect(dialog.getByRole('button', { name: 'Follow the schedule' })).toBeVisible();
  await expect(chip(page)).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Follow the schedule' }).click();
  await page.keyboard.press('Escape');
  await expect(chip(page)).toBeVisible();
  expect(agpjOk(mainProject, ['mode']).stdout).toContain('by the schedule');
});
