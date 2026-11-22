import { expect, test, type Page } from '../helpers/fixtures';
import { agpjOk, askArgs, decisionJson, mainProject, projectId, nextTaskId, resetBoard, type TaskJson } from '../helpers/agpj';
import { waitForSync } from '../helpers/ui';
import { answerButton, fillAnswer, send, openAnswer } from '../helpers/answer';
import { closeMessages } from '../helpers/answer-flow';

// An AI card in Todo can ask a decision before the work starts. It shows as pending in Todo, answers there, and
// returns to Todo idle. A move keeps the question open; withdrawing it is what ends the wait.

const details = (page: Page) => page.getByRole('complementary', { name: 'Task details' });
const task = (page: Page, id: number) => page.getByRole('button', { name: new RegExp(`^#${id}(?!\\d)`) });
const lane = (page: Page, name: string) => page.getByRole('region', { name });

function show(id: number): TaskJson & { taskStatus: string; agentStatus: string; pendingRequestId: string | null } {
  return JSON.parse(agpjOk(mainProject, ['show', String(id), '--json']).stdout).task;
}

test.beforeEach(async ({ page }) => {
  resetBoard(mainProject);
  await page.goto(`/?project=${projectId(mainProject)}`);
  await waitForSync(page);
});

test('an AI task in Todo asks, shows as pending there, and the answer returns it to Todo idle', async ({ page }) => {
  const id = nextTaskId(mainProject);
  agpjOk(mainProject, ['add', 'Ask before starting', '--assignee', 'ai', '--actor', 'human']);
  agpjOk(mainProject, askArgs(String(id), decisionJson('Which plan before we start?')));
  await page.reload();
  await waitForSync(page);

  // In Todo the card shows its pending state and the answer button, like a card in Doing; the question is in the dialog.
  await expect(lane(page, 'Todo').getByRole('button', { name: new RegExp(`^#${id}(?!\\d)`) })).toContainText('Awaiting answer');
  await expect(answerButton(page, id)).toBeVisible();
  await expect(lane(page, 'Todo')).not.toContainText('Which plan before we start?');

  await task(page, id).click();
  const dialog = await openAnswer(page, id);
  await expect(dialog).toContainText('Which plan before we start?');
  await fillAnswer(dialog, { choice: 'Plan A', text: 'Plan A, then review' });
  await send(page);
  await closeMessages(page);
  await expect(answerButton(page, id)).toHaveCount(0);

  // The card is still in Todo, and idle again.
  expect(show(id)).toMatchObject({ taskStatus: 'todo', agentStatus: 'idle', pendingRequestId: null });
  await expect(lane(page, 'Todo').getByRole('button', { name: new RegExp(`^#${id}(?!\\d)`) })).not.toContainText('Awaiting answer');
});

test('moving a task out of Todo keeps its question pending until the question is withdrawn', async ({ page }) => {
  const id = nextTaskId(mainProject);
  agpjOk(mainProject, ['add', 'Move while asking', '--assignee', 'ai', '--actor', 'human']);
  const asked = JSON.parse(agpjOk(mainProject, ['--json', ...askArgs(String(id), decisionJson('Still wanted?'))]).stdout.split('\n')[0]) as {
    questions: { id: string }[];
  };
  const question = asked.questions[0].id;
  await page.reload();
  await waitForSync(page);
  const card = () => lane(page, 'Doing').getByRole('button', { name: new RegExp(`^#${id}(?!\\d)`) });

  // The lane move leaves the question open: the card waits in Doing on the same question.
  agpjOk(mainProject, ['mv', String(id), 'doing', '--actor', 'human', '--reason', 'Start now.']);
  await page.reload();
  await waitForSync(page);
  expect(show(id)).toMatchObject({ taskStatus: 'doing', agentStatus: 'awaiting_answer', pendingRequestId: question });
  await expect(card()).toContainText('Awaiting answer');
  await expect(answerButton(page, id)).toBeVisible();

  // Withdrawing the question ends the wait.
  agpjOk(mainProject, ['message', 'withdraw', question, '--actor', 'ai']);
  await page.reload();
  await waitForSync(page);
  expect(show(id)).toMatchObject({ taskStatus: 'doing', agentStatus: 'idle', pendingRequestId: null });
  await expect(card()).not.toContainText('Awaiting answer');
  await expect(answerButton(page, id)).toHaveCount(0);
});
