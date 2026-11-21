import { agboardOk, listTasks, mainProject, resetBoard } from '../helpers/agboard';
import { expect, type Page, test } from '../helpers/fixtures';

// Every way the phone's task drawer closes must release the hold on automatic updates: the card
// follows the next edit about 3 seconds later. Driven with a mouse at phone width.
test.use({ viewport: { width: 390, height: 844 } });
test.beforeEach(() => resetBoard(mainProject));

const closers: Record<string, (page: Page) => Promise<void>> = {
  'the close button': (page) => page.getByRole('button', { name: 'Close details', exact: true }).click(),
  Escape: (page) => page.keyboard.press('Escape'),
  'a tap on the backdrop': (page) => page.mouse.click(195, 20),
  'a swipe down on the handle': async (page) => {
    // The sheet is still sliding in right after it opens; its handle moves until it rests.
    await page.waitForTimeout(800);
    const box = (await page.getByRole('button', { name: 'Swipe down to close details' }).boundingBox())!;
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;
    await page.mouse.move(x, y);
    await page.mouse.down();
    for (let i = 1; i <= 15; i += 1) await page.mouse.move(x, y + i * 40);
    await page.mouse.up();
  },
  'the ArrowDown key on the handle': async (page) => {
    await page.getByRole('button', { name: 'Swipe down to close details' }).focus();
    await page.keyboard.press('ArrowDown');
  },
};

for (const [name, close] of Object.entries(closers)) {
  test(`updates follow again after the drawer closes by ${name}`, async ({ page }) => {
    const task = listTasks(mainProject).find((t) => t.taskStatus === 'todo')!;
    await page.goto('/');
    await page.getByRole('tab', { name: /^Todo/ }).click();
    const card = (title: string) =>
      page.getByRole('tabpanel', { name: /^Todo/ }).getByRole('button', { name: title, exact: true });
    await card(task.title).click();
    const drawer = page.getByRole('dialog');
    await expect(drawer).toBeVisible();
    await close(page);
    await expect(drawer).toBeHidden();
    agboardOk(mainProject, ['edit', String(task.id), '--title', `After ${name}`]);
    await expect(card(`After ${name}`)).toBeVisible({ timeout: 9_000 });
  });
}
