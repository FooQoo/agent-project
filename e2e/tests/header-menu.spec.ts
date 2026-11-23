import { expect, test } from '../helpers/fixtures';
import { mainProject, resetBoard } from '../helpers/agpj';
import { openHeaderMenu } from '../helpers/ui';

// The PC header keeps the product name, the project, the version and Ask AI; Docs, Epics, Archive and
// Settings are in one menu at its right end.
test.beforeEach(async ({ page }) => {
  resetBoard(mainProject);
  await page.goto('/');
  await expect(page.getByRole('contentinfo').getByRole('status')).toHaveText(/^Synced/);
});

for (const viewport of [
  { width: 1180, height: 820 },
  { width: 820, height: 1180 },
  { width: 1440, height: 900 },
]) {
  test(`${viewport.width}x${viewport.height}: nothing in the header is cut short`, async ({ page }) => {
    await page.setViewportSize(viewport);
    const header = page.getByRole('banner');
    await expect(header.getByRole('button', { name: /^Version filter:/ })).toBeVisible();
    await expect(header.getByRole('button', { name: 'Ask AI' })).toBeVisible();
    await expect(header.getByRole('button', { name: 'Menu', exact: true })).toBeVisible();
    // No label of the header is clipped: every box shows its whole content.
    const clipped = await header.evaluate((root) =>
      [...root.querySelectorAll<HTMLElement>('*')]
        .filter((element) => element.scrollWidth > element.clientWidth + 0.5 && element.clientWidth > 0)
        .map((element) => element.className || element.tagName),
    );
    expect(clipped).toEqual([]);
    expect(await header.evaluate((root) => root.scrollWidth - root.clientWidth)).toBeLessThanOrEqual(0);
  });
}

test('the version button shows the flag and the version, named as the version filter for readers', async ({ page }) => {
  const button = page.getByRole('banner').getByRole('button', { name: /^Version filter:/ });
  await expect(button).toHaveAccessibleName('Version filter: All');
  await expect(button).toHaveText('All');
});

test('the menu holds Docs, Epics, Archive and Settings, in that order', async ({ page }) => {
  const menu = await openHeaderMenu(page);
  await expect(menu.getByRole('menuitem')).toHaveText(['Docs', 'Epics', 'Archive', 'Settings']);
  for (const item of await menu.getByRole('menuitem').all()) {
    expect((await item.boundingBox())!.height).toBeGreaterThanOrEqual(36);
  }
  await page.keyboard.press('Escape');
  await expect(page.getByRole('menu')).toHaveCount(0);
  await expect(page.getByRole('banner').getByRole('button', { name: 'Menu', exact: true })).toBeFocused();
});
