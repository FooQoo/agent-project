import { expect, test, type Page } from '../helpers/fixtures';
import { agpjOk, mainProject, mainUrl, nextTaskId, questionJson, resetBoard } from '../helpers/agpj';
import { answerDialog, messageList, openMessageList } from '../helpers/answer';
import { resetPreferences } from '../helpers/preferences';
import { openHeaderItem, waitForSync } from '../helpers/ui';

// Brainstorm mode on the board: it starts from a card or an epic, the header chip names the target and ends
// it, a question asked with --brainstorm shows which target it is about, and sleep wins over it.

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
const taskButton = (page: Page, id: number) =>
  page.getByRole('button', { name: new RegExp(`^#${id}(?!\\d)`) });

const ask = (body: string) =>
  agpjOk(mainProject, [
    'message', 'ask', '--actor', 'ai', '--brainstorm', '--no-wait',
    '--questions-json', questionJson(body, []),
  ]);

test('brainstorming starts from a card, names its target in the chip and the question, and ends from the chip', async ({
  page,
}) => {
  const id = nextTaskId(mainProject);
  agpjOk(mainProject, ['add', 'Brainstorm this card', '--assignee', 'ai', '--actor', 'human']);
  await page.goto(mainUrl());
  await waitForSync(page);
  await taskButton(page, id).click();
  const details = page.getByRole('complementary', { name: 'Task details' });
  await details.getByRole('button', { name: `Brainstorm #${id}` }).click();

  await expect(chip(page)).toContainText(`Brainstorm: #${id}`);
  await expect(details.getByText('Brainstorming', { exact: true })).toBeVisible();
  expect(agpjOk(mainProject, ['mode']).stdout.toLowerCase()).toContain('brainstorm');

  // A question asked with --brainstorm says which target it is about.
  ask('Who reads this card first?');
  const list = await openMessageList(page);
  await expect(list).toBeVisible();
  await messageList(page).getByRole('button', { name: /Who reads this card first/ }).click();
  await expect(answerDialog(page).getByText(`Brainstorm: #${id}`)).toBeVisible();
  await page.keyboard.press('Escape');

  // The chip ends it; the project goes back to sparring.
  await page.getByRole('banner').getByRole('button', { name: 'End brainstorm' }).click();
  await expect(chip(page)).toHaveCount(0);
  expect(agpjOk(mainProject, ['mode']).stdout).toContain('sparring');
});

test('brainstorming starts from an epic, moves to another target, and ends in the settings', async ({ page }) => {
  agpjOk(mainProject, ['epic', 'add', 'BRAINEPIC', '--name', 'Brainstorm epic sample', '--actor', 'ai']);
  const id = nextTaskId(mainProject);
  agpjOk(mainProject, ['add', 'Other target card', '--assignee', 'ai', '--actor', 'human']);
  await page.goto(mainUrl());
  await waitForSync(page);

  await openHeaderItem(page, 'Epics');
  await page.getByRole('list', { name: 'Epics in this project' }).getByRole('listitem')
    .filter({ hasText: 'Brainstorm epic sample' }).getByRole('button').first().click();
  await page.getByRole('button', { name: 'Brainstorm BRAINEPIC' }).click();
  await expect(chip(page)).toContainText('Brainstorm: BRAINEPIC');

  // Starting on another target moves the brainstorm: there is one target at a time.
  agpjOk(mainProject, ['mode', 'brainstorm', '--target', String(id)]);
  await expect(chip(page)).toContainText(`Brainstorm: #${id}`, { timeout: 15_000 });

  await openHeaderItem(page, 'Settings');
  const dialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(dialog.getByText(`Brainstorming #${id}`)).toBeVisible();
  await dialog.getByRole('button', { name: 'End brainstorm' }).click();
  await expect(dialog.getByText(`Brainstorming #${id}`)).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(chip(page)).toHaveCount(0);
});

test('sleep disables the Brainstorm action and replaces a brainstorm', async ({ page }) => {
  const id = nextTaskId(mainProject);
  agpjOk(mainProject, ['add', 'Sleeping card', '--assignee', 'ai', '--actor', 'human']);
  agpjOk(mainProject, ['mode', 'sleep']);
  await page.goto(mainUrl());
  await waitForSync(page);
  await taskButton(page, id).click();
  const details = page.getByRole('complementary', { name: 'Task details' });
  await expect(details.getByRole('button', { name: `Brainstorm #${id}` })).toBeDisabled();
  await expect(details.getByText('Brainstorming is unavailable in sleep mode.')).toBeAttached();

  // Back to sparring the action works again. The CLI can put the project to sleep.
  agpjOk(mainProject, ['mode', 'sparring']);
  await expect(details.getByRole('button', { name: `Brainstorm #${id}` })).toBeEnabled({ timeout: 15_000 });
  await details.getByRole('button', { name: `Brainstorm #${id}` }).click();
  await expect(chip(page)).toContainText(`Brainstorm: #${id}`);
  agpjOk(mainProject, ['mode', 'sleep']);
  await expect(chip(page)).toContainText('Sleep', { timeout: 15_000 });
  await expect(chip(page)).not.toContainText('Brainstorm');
});
