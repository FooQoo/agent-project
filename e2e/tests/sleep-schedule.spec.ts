import { expect, test, type Page } from '../helpers/fixtures';
import { agpjOk, mainProject, mainUrl, resetBoard } from '../helpers/agpj';
import { resetPreferences } from '../helpers/preferences';
import { openHeaderItem } from '../helpers/ui';

test.beforeEach(async () => {
  resetBoard(mainProject);
  await resetPreferences();
  agpjOk(mainProject, ['mode', 'schedule', '--start', clock(-4), '--end', clock(4)]);
  agpjOk(mainProject, ['mode', 'schedule', '--off']);
  agpjOk(mainProject, ['mode', 'auto']);
});

test.afterEach(() => {
  agpjOk(mainProject, ['mode', 'schedule', '--off']);
  agpjOk(mainProject, ['mode', 'auto']);
});

const readMode = () => JSON.parse(agpjOk(mainProject, ['mode', '--json']).stdout);
const chip = (page: Page) => page.getByRole('status', { name: /^Sleep mode:/ });

async function openSettings(page: Page, phone = false) {
  if (phone) await page.getByRole('button', { name: 'Settings', exact: true }).click();
  else await openHeaderItem(page, 'Settings');
  const dialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(dialog).toBeVisible();
  return dialog;
}

// The e2e servers and commands run with TZ=UTC, so the window is written in UTC.
function clock(offsetHours: number) {
  const time = new Date(Date.now() + offsetHours * 3_600_000);
  return `${String(time.getUTCHours()).padStart(2, '0')}:${String(time.getUTCMinutes()).padStart(2, '0')}`;
}

test('a sleep schedule saved on the PC updates both headers and can be turned off on the phone', async ({
  page,
  browser,
}) => {
  const phone = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true });
  try {
    const phonePage = await phone.newPage();
    await Promise.all([page.goto(mainUrl()), phonePage.goto(mainUrl())]);
    const settings = await openSettings(page);
    const start = clock(-2);
    const end = clock(2);
    const from = settings.getByLabel('From');
    const until = settings.getByLabel('Until');

    await from.fill(start);
    await expect.poll(() => readMode().schedule.start).toBe(start);
    await until.fill(end);
    await expect.poll(() => readMode().schedule.end).toBe(end);
    const enabled = settings.getByRole('checkbox', { name: 'Sleep on a schedule' });
    await enabled.click();
    await expect(enabled).toBeChecked();
    await page.keyboard.press('Escape');
    await expect(chip(page)).toContainText('Sleep');
    await expect(chip(page).getByText(/^until /)).toBeVisible();
    await expect(chip(phonePage)).toContainText('Sleep', { timeout: 15_000 });
    expect(agpjOk(mainProject, ['mode']).stdout).toContain('by the schedule');

    const phoneSettings = await openSettings(phonePage, true);
    await expect(phoneSettings.getByLabel('From')).toHaveValue(start);
    await expect(phoneSettings.getByLabel('Until')).toHaveValue(end);
    const phoneEnabled = phoneSettings.getByRole('checkbox', { name: 'Sleep on a schedule' });
    await phoneEnabled.tap();
    await expect(phoneEnabled).not.toBeChecked();
    await phonePage.keyboard.press('Escape');
    await expect(chip(phonePage)).toHaveCount(0);
    await expect(chip(page)).toHaveCount(0, { timeout: 15_000 });
  } finally {
    await phone.close();
  }
});

test('a CLI override changes the open board, and settings can return control to the schedule', async ({
  page,
}) => {
  agpjOk(mainProject, ['mode', 'schedule', '--start', clock(-2), '--end', clock(2)]);
  await page.goto(mainUrl());
  await expect(chip(page)).toContainText('Sleep');

  agpjOk(mainProject, ['mode', 'sparring']);
  await expect(chip(page)).toHaveCount(0, { timeout: 15_000 });
  const dialog = await openSettings(page);
  await dialog.getByRole('button', { name: 'Follow the schedule' }).click();
  await page.keyboard.press('Escape');
  await expect(chip(page)).toContainText('Sleep');
  expect(agpjOk(mainProject, ['mode']).stdout).toContain('by the schedule');
});

test('settings can end manual sleep without enabling a schedule', async ({ page }) => {
  agpjOk(mainProject, ['mode', 'sleep']);
  await page.goto(mainUrl());
  await expect(chip(page)).toContainText('Sleep');

  const dialog = await openSettings(page);
  await expect(dialog.getByRole('checkbox', { name: 'Sleep on a schedule' })).not.toBeChecked();
  await dialog.getByRole('button', { name: 'End sleep', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(chip(page)).toHaveCount(0);
});
