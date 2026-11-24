import { readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { agpjOk, createEmptyProject, mainHome, projectId, replaceFileAtomically, workspace } from '../helpers/agpj';
import { expect, test } from '../helpers/fixtures';
import { openHeaderItem } from '../helpers/ui';

test('a person can register an agent and reassign a Doing card on PC and phone', async ({ page }) => {
  const project = 'Assignment UI';
  createEmptyProject(mainHome(), project);
  const { task } = JSON.parse(
    agpjOk(project, ['add', 'Review the release', '--assignee', 'ai', '--actor', 'human', '--json']).stdout,
  ) as { task: { id: number } };
  agpjOk(project, ['mv', String(task.id), 'doing', '--actor', 'human']);

  const target = `/?project=${projectId(project)}`;
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(target);
  await openHeaderItem(page, 'Settings');
  await page.getByRole('region', { name: 'Agents', exact: true }).getByRole('button', { name: 'Manage' }).click();
  const settings = page.getByRole('dialog', { name: 'Agents', exact: true });
  await settings.getByRole('button', { name: 'Add agent' }).click();
  await settings.getByLabel('Name').fill('Release reviewer');
  await settings.getByLabel('Role').fill('Checks release notes');
  await settings.getByRole('button', { name: 'Save' }).click();
  await expect(settings.getByText('Release reviewer')).toBeVisible();
  await expect(settings.getByText('Checks release notes')).toBeVisible();
  await settings.getByRole('button', { name: 'Close', exact: true }).click();

  const desktopCard = page.locator('.card[data-lane="doing"]').filter({ hasText: 'Review the release' });
  await expect(desktopCard.getByTitle('Agent', { exact: true })).toBeVisible();
  await desktopCard.click();
  await page.getByRole('button', { name: 'Change agent · Agent', exact: true }).click();
  const desktopOption = page.getByRole('menuitemradio', { name: /Release reviewer/ });
  await expect(desktopOption).toContainText('Checks release notes');
  await desktopOption.click();
  await expect(page.getByRole('button', { name: 'Change agent · Release reviewer' })).toBeVisible();
  await expect(desktopCard.getByTitle('Release reviewer')).toBeVisible();
  await expect(desktopCard).toHaveAccessibleDescription('Release reviewer');

  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(target);
  await page.getByRole('tab', { name: /^Doing/ }).click();
  const phoneCard = page
    .getByRole('tabpanel', { name: /^Doing/ })
    .getByRole('article', { name: /Review the release/ });
  await expect(phoneCard.getByTitle('Release reviewer')).toBeVisible();
  await expect(phoneCard).toHaveAccessibleDescription('Release reviewer');
  await phoneCard.getByRole('button', { name: 'Review the release' }).click();
  await page.getByRole('button', { name: 'Change agent · Release reviewer' }).click();
  const picker = page.getByRole('dialog', { name: 'Change agent' });
  await expect(picker.getByRole('button', { name: /Release reviewer/ })).toContainText('Checks release notes');
  await picker.getByRole('button', { name: /^Agent Disconnected$/ }).click();
  await expect(picker).toBeHidden();
  await expect(page.getByRole('button', { name: 'Change agent · Agent', exact: true })).toBeVisible();
  const taskState = JSON.parse(agpjOk(project, ['show', String(task.id), '--json']).stdout);
  expect(taskState.task).toMatchObject({ taskStatus: 'doing', agentId: 'A-1' });
});

test('an existing card can use a default agent set up in Settings', async ({ page }) => {
  const project = 'Existing assignments';
  createEmptyProject(mainHome(), project);
  const { task } = JSON.parse(
    agpjOk(project, ['add', 'Existing AI card', '--assignee', 'ai', '--actor', 'human', '--json']).stdout,
  ) as { task: { id: number } };
  const folder = workspace(project).folder;
  const metadataPath = join(folder, 'tasks', String(task.id), 'task.json');
  const metadata = JSON.parse(readFileSync(metadataPath, 'utf8'));
  replaceFileAtomically(metadataPath, JSON.stringify({ ...metadata, agentId: null }));
  rmSync(join(folder, 'agents.json'));

  await page.goto(`/?project=${projectId(project)}`);
  await openHeaderItem(page, 'Settings');
  await page.getByRole('region', { name: 'Agents', exact: true }).getByRole('button', { name: 'Manage' }).click();
  const settings = page.getByRole('dialog', { name: 'Agents', exact: true });
  await settings.getByRole('button', { name: 'Set up default agent' }).click();
  await settings.getByRole('button', { name: 'Connect', exact: true }).click();
  await settings.getByRole('button', { name: 'Issue connection code' }).click();
  const command = settings.locator('code');
  await expect(command).toHaveText(/^agpj connect [A-Z2-9]+$/);
  const code = (await command.innerText()).split(' ')[2];
  agpjOk(project, ['connect', code]);
  agpjOk(project, ['mv', String(task.id), 'doing', '--actor', 'ai']);
  agpjOk(project, ['state', String(task.id), 'running', '--actor', 'ai']);
  await page.keyboard.press('Escape');
  const card = page.locator('.card[data-lane="doing"]').filter({ hasText: 'Existing AI card' });
  await expect(card.getByTitle('Agent', { exact: true })).toBeVisible();
});
