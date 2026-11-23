import { expect, test } from '../helpers/fixtures';
import { mainProject, resetBoard } from '../helpers/agpj';
import { readBoardAfterReset, openHeaderItem } from '../helpers/ui';

// The "Notifications" row of the settings dialog opens a dialog of its own. The choices are
// saved at once through the real server, and the settings list no longer holds them.

test.beforeEach(async ({ page }) => {
  resetBoard(mainProject);
  await page.goto('/');
  await expect(page.getByRole('contentinfo').getByRole('status')).toHaveText(/^Synced/);
  await readBoardAfterReset(page);
});

test('the row opens the notification dialog, a choice is saved, and focus returns to Settings', async ({ page }) => {
  await openHeaderItem(page, 'Settings');
  const settings = page.getByRole('dialog', { name: 'Settings' });
  // The settings list has the row, not the notification checkboxes. (The Mode section has its own
  // schedule checkbox, so only the notification choices are counted.)
  await expect(
    settings.getByRole('checkbox', { name: /^(Results|Questions that need|History additions)/ }),
  ).toHaveCount(0);
  await settings.getByRole('button', { name: 'Notification settings' }).click();
  await expect(settings).toHaveCount(0);

  const dialog = page.getByRole('dialog', { name: 'Notifications' });
  await expect(dialog).toBeVisible();
  // The defaults: results only.
  await expect(dialog.getByRole('checkbox', { name: /^Results/ })).toBeChecked();
  await expect(dialog.getByRole('checkbox', { name: /^Questions that need/ })).not.toBeChecked();
  const history = dialog.getByRole('checkbox', { name: /^History additions/ });
  await expect(history).not.toBeChecked();
  await history.click();
  await expect(history).toBeChecked();

  // Saved on the server: a reload shows the same choice.
  await expect
    .poll(async () => (await page.request.get('/api/notification-settings')).ok())
    .toBe(true);
  await page.reload();
  await openHeaderItem(page, 'Settings');
  await page.getByRole('button', { name: 'Notification settings' }).click();
  await expect(
    page.getByRole('dialog', { name: 'Notifications' }).getByRole('checkbox', { name: /^History additions/ }),
  ).toBeChecked();

  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Notifications' })).toHaveCount(0);
});
