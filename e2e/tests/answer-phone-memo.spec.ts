import { agpjOk, listTasks, mainProject, resetBoard } from '../helpers/agpj';
import { listRow, messagesItem, openMessageList } from '../helpers/answer';
import { expect, test } from '../helpers/fixtures';
import { resetPreferences, setPreferences } from '../helpers/preferences';

test.use({ hasTouch: true, isMobile: true });

// Positions are compared with 2px of room for subpixel rounding.
const tolerance = 2;

// A question long enough that the memo field starts far below the first screen of the sheet.
const section = (n: number) =>
  `## 論点 ${n}\n\n既存ユーザーの扱いと、移行の順番、問い合わせの増え方をどう見積もるかを決めてください。詳しくは [設計メモ ${n}](https://example.com/${n}) を参照。\n\n- 初回ログイン時に変換する\n- 再設定をお願いする\n`;
const question = `# 認証方式の選び方\n\n${Array.from({ length: 10 }, (_, i) => section(i + 1)).join('\n')}`;

for (const size of [
  { width: 390, height: 844, keyboard: 336 },
  { width: 320, height: 568, keyboard: 260 },
]) {
  test.describe(`${size.width}px wide`, () => {
    test.use({ viewport: { width: size.width, height: size.height } });

    test.beforeEach(async ({ page }) => {
      resetBoard(mainProject);
      await setPreferences({ locale: 'ja', theme: { mode: process.env.AB_DARK ? 'dark' : 'light', accent: 'blue' } });
      // iOS Safari keeps the layout viewport at full height and only shrinks the visual viewport when the
      // keyboard opens; a stand-in for it lets the test play that.
      await page.addInitScript(() => {
        const target = new EventTarget();
        const state = { shrink: 0 };
        const root = () => document.documentElement;
        const fake = Object.defineProperties(target, {
          height: { get: () => root().clientHeight - state.shrink },
          offsetTop: { get: () => 0 },
          width: { get: () => root().clientWidth },
        });
        Object.defineProperty(window, 'visualViewport', { value: fake, configurable: true });
        (window as unknown as { setKeyboard: (height: number) => void }).setKeyboard = (height) => {
          state.shrink = height;
          target.dispatchEvent(new Event('resize'));
        };
      });
      const id = listTasks(mainProject).find((task) => task.taskStatus === 'doing' && task.assignee === 'ai')!.id;
      agpjOk(mainProject, [
        'message', 'ask', '--actor', 'ai', '--no-wait', '--task', String(id), '--questions-json',
        JSON.stringify({ questions: [{ body: question, choices: [{ id: 'a', label: 'メールとパスワードだけ' }, { id: 'b', label: '外部アカウントも足す' }] }] }),
      ]);
      await page.goto('/');
      await expect(messagesItem(page)).toBeVisible();
    });

    test.afterEach(() => resetPreferences());

    for (const where of ['top', 'middle', 'bottom'] as const) {
      test(`the memo field stays fully visible when focused with the sheet scrolled to the ${where}`, async ({ page }) => {
        await openMessageList(page);
        await listRow(page, /認証方式の選び方/).click();
        const sheet = page.getByRole('dialog', { name: 'メッセージ' });
        const memo = sheet.getByRole('textbox', { name: 'メモ' });
        const body = sheet.locator('.surface__body');
        const footer = sheet.locator('.surface__footer');
        await expect(footer.getByRole('button', { name: '回答を送る' })).toBeVisible();
        const settle = () =>
          expect(async () => {
            const first = await sheet.boundingBox();
            await page.waitForTimeout(100);
            expect(await sheet.boundingBox()).toEqual(first);
          }).toPass();
        await settle();
        expect(await body.evaluate((el) => el.scrollHeight - el.clientHeight)).toBeGreaterThan(size.height);

        await body.evaluate((el, at) => {
          el.scrollTop = at === 'top' ? 0 : at === 'middle' ? (el.scrollHeight - el.clientHeight) / 2 : el.scrollHeight;
        }, where);
        // The field takes the focus without the browser's own scroll, then the keyboard comes up.
        await memo.evaluate((el) => (el as HTMLTextAreaElement).focus({ preventScroll: true }));
        await page.evaluate((h) => (window as unknown as { setKeyboard: (height: number) => void }).setKeyboard(h), size.keyboard);
        await settle();
        await page.waitForTimeout(500);

        const visible = async () => {
          const field = await memo.boundingBox();
          const area = await body.boundingBox();
          const bar = await footer.boundingBox();
          if (!field || !area || !bar) throw new Error('The sheet did not render.');
          return { field, area, bar };
        };
        const { field, area, bar } = await visible();
        if (process.env.AB_SHOTS) await page.screenshot({ path: `${process.env.AB_SHOTS}/memo-${size.width}-${where}.png` });
        expect(field.y).toBeGreaterThanOrEqual(area.y - tolerance);
        expect(field.y + field.height).toBeLessThanOrEqual(area.y + area.height + tolerance);
        expect(field.y + field.height).toBeLessThanOrEqual(bar.y + tolerance);
        expect(bar.y + bar.height).toBeLessThanOrEqual(size.height - size.keyboard + tolerance);

        // Typing more lines keeps the line being typed in sight.
        for (let i = 0; i < 12; i++) await page.keyboard.type(`行 ${i}\n`);
        await page.waitForTimeout(300);
        const typed = await visible();
        expect(typed.field.y + typed.field.height).toBeLessThanOrEqual(typed.bar.y + tolerance);
        expect(typed.field.y + typed.field.height).toBeGreaterThan(typed.area.y);
        expect(typed.field.y).toBeLessThanOrEqual(typed.area.y + typed.area.height);
      });
    }
  });
}
