import { mkdtempSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { agboard, agpjCatalogOk, projectId, skipGuide, taskBody } from '../helpers/agpj';
import { expect, test } from '../helpers/fixtures';
import { startPhoneEntrance, type PhoneEntrance } from '../helpers/monitor';

// The phone screen keeps following the change feed after a task drawer has been opened and closed.
// The drawer holds automatic updates while it is open and for 3 seconds after it closes, and what
// arrived meanwhile is applied then; neither may leave the page stuck. Runs on the phone entrance,
// where the phone screen has its own data layer, and on the board below 768px.
test.describe.configure({ mode: 'serial' });
test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

let home: string;
let entrance: PhoneEntrance;
let id: number;

test.beforeAll(async () => {
  test.setTimeout(240_000);
  home = mkdtempSync(path.join(os.tmpdir(), 'agent-board-phone-drawer-'));
  skipGuide(home);
  agpjCatalogOk(['project', 'create', 'Live'], home);
  const added = JSON.parse(
    agboard('Live', ['add', 'Live card', `--desc=${taskBody('Body.')}`, '--assignee', 'ai', '--actor', 'human', '--json'], home).stdout,
  ) as { task: { id: number } };
  id = added.task.id;
  entrance = await startPhoneEntrance(home);
});

test.afterAll(async () => {
  await entrance?.stop();
  if (home) rmSync(home, { recursive: true, force: true });
});

test('the phone screen follows edits made before, during and after an open drawer', async ({ page }) => {
  const edit = (title: string) => {
    const result = agboard('Live', ['edit', String(id), '--title', title], home);
    if (result.code !== 0) throw new Error(result.stderr);
  };
  await entrance.open(page, `${entrance.phoneOrigin}/?project=${projectId('Live', home)}`);
  await page.getByRole('tab', { name: /^Todo/ }).click();
  const card = (title: string) =>
    page.getByRole('tabpanel', { name: /^Todo/ }).getByRole('button', { name: title, exact: true });
  await expect(card('Live card')).toBeVisible();

  edit('Before the drawer');
  await expect(card('Before the drawer')).toBeVisible({ timeout: 8_000 });

  for (const round of [1, 2]) {
    await card(round === 1 ? 'Before the drawer' : `After close ${round - 1}`).click();
    const drawer = page.getByRole('dialog');
    await expect(drawer).toBeVisible();
    edit(`While open ${round}`);
    await page.waitForTimeout(4_000);
    await drawer.getByRole('button', { name: 'Close details', exact: true }).click();
    await expect(drawer).toBeHidden();
    // What arrived while the drawer was open is applied once the 3 second hold has ended.
    await expect(card(`While open ${round}`)).toBeVisible({ timeout: 8_000 });
    edit(`After close ${round}`);
    await expect(card(`After close ${round}`)).toBeVisible({ timeout: 8_000 });
  }
});
