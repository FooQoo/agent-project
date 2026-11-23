import type { Page } from '@playwright/test';
import { expect, test } from '../helpers/fixtures';
import { agpjOk, mainProject, nextTaskId, resetBoard } from '../helpers/agpj';
import { resetPreferences } from '../helpers/preferences';
import { clearVersions, releaseVersion, plannedVersions, planTask, seedVersions } from '../helpers/versions';
import { waitForSync, openHeaderItem } from '../helpers/ui';

// #646: planned versions on the PC board: the version filter in the header, the tag on a card, the planned
// version in the task's edit dialog, and an epic's tasks planned together in the epic's edit dialog. The phone
// shows the tag, filters by version and lists a new version, but cannot change the release version.
test.beforeEach(() => resetPreferences());
test.afterEach(() => {
	clearVersions();
	return resetPreferences();
});

interface Seed {
	/** The LOGIN epic's tasks: A is planned for v1.8.0, B, C and D are not (D is in Doing). */
	a: number;
	b: number;
	c: number;
	d: number;
	/** A Todo card of no epic, planned for v1.9.0. */
	e: number;
}

/** Two versions, the card layout above, and no release version yet. */
async function seed(release: string | null = null): Promise<Seed> {
	clearVersions();
	resetBoard(mainProject);
	agpjOk(mainProject, ['epic', 'add', 'LOGIN', '--name', 'Login work', '--actor', 'human']);
	const first = nextTaskId(mainProject);
	const ids = [0, 1, 2, 3, 4].map((offset) => first + offset);
	const add = (title: string, epic?: string) =>
		agpjOk(mainProject, ['add', title, '--assignee', 'ai', '--actor', 'human', ...(epic ? ['--epic', epic] : [])]);
	add('Plan A login screen', 'LOGIN');
	add('Plan B reset mail', 'LOGIN');
	add('Plan C session', 'LOGIN');
	add('Plan D history', 'LOGIN');
	add('Plan E dark theme');
	agpjOk(mainProject, ['mv', String(ids[3]), 'doing', '--actor', 'human', '--reason', 'Start']);
	await seedVersions(['v1.8.0', 'v1.9.0'], release);
	await planTask(ids[0] as number, 'v1.8.0');
	await planTask(ids[4] as number, 'v1.9.0');
	const [a, b, c, d, e] = ids as [number, number, number, number, number];
	return { a, b, c, d, e };
}

const card = (page: Page, title: string) => page.locator('.card', { hasText: title });

/** Opens a task's edit dialog from its card. The board may read the project again right after a spec's setup, so the opening is tried again. */
async function openTaskDialog(page: Page, title: string, id: number) {
	const open = page.getByLabel('Task details').getByRole('button', { name: 'Open full screen (task)', exact: true });
	await expect(async () => {
		await card(page, title).click();
		await open.click({ timeout: 3000 });
	}).toPass();
	return page.getByRole('dialog', { name: new RegExp(`^#${id} Task preview`) });
}
const headerMenu = (page: Page) => page.getByRole('button', { name: /^Version filter:/ });

test('filters the board by a version, makes it the release version from the header, plans a task and plans an epic in their edit dialogs', async ({ page }) => {
	const cards = await seed();
	await page.setViewportSize({ width: 1440, height: 900 });
	await page.goto('/');
	await waitForSync(page);

	// The version filter is a menu in the header; with no release version it starts on All. A project that lists versions shows the tag on every card.
	await expect(headerMenu(page)).toHaveAccessibleName('Version filter: All');
	await expect(card(page, 'Plan A login screen').locator('.ver-tag')).toContainText('v1.8.0');
	await expect(card(page, 'Plan C session').locator('.ver-tag')).toContainText('Unplanned');
	// The tag shares the date row: it is at the left of the date, which has no "Updated" word. The acceptance
	// chip is in the top row, not in the footer.
	for (const title of ['Plan A login screen', 'Plan C session']) {
		const foot = card(page, title).locator('.card__foot');
		await expect(foot.locator('.ver-tag')).toHaveCount(1);
		await expect(foot.locator('.card__updated')).toHaveText(/^\d{2}\/\d{2} \d{2}:\d{2}$/);
		await expect(foot.locator('.acceptance-chip')).toHaveCount(0);
		const [tag, updated] = await Promise.all([foot.locator('.ver-tag').boundingBox(), foot.locator('.card__updated').boundingBox()]);
		expect(tag!.x + tag!.width).toBeLessThanOrEqual(updated!.x);
		expect(Math.abs(tag!.y + tag!.height / 2 - (updated!.y + updated!.height / 2))).toBeLessThan(6);
	}

	// Type a new version and confirm: it is listed, and neither the filter nor the release version changes.
	await headerMenu(page).click();
	const confirm = page.getByRole('button', { name: 'Add this version to the project' });
	await expect(confirm).toBeDisabled();
	await page.getByLabel('New version').fill('v2.0.0');
	await confirm.click();
	await expect(page.getByRole('radio', { name: /^v2\.0\.0/ })).toBeVisible();
	await expect(headerMenu(page)).toHaveAccessibleName('Version filter: All');
	expect(await releaseVersion()).toBeNull();
	// All is first, then the versions newest first; there is no "No release version" item and no release button while All is shown.
	const listed = await page.getByRole('radio').evaluateAll((radios) => radios.map((radio) => radio.parentElement?.textContent ?? ''));
	expect(listed).toEqual(['All', 'v2.0.0', 'v1.9.0', 'v1.8.0']);
	await expect(page.getByRole('button', { name: /the release version$/ })).toHaveCount(0);

	// Choose a version: the Todo, Doing and Done lanes show that version's cards and the unplanned ones. Nothing is saved.
	await page.getByRole('radio', { name: /^v1\.8\.0/ }).click();
	await expect(headerMenu(page)).toHaveAccessibleName('Version filter: v1.8.0');
	expect(await releaseVersion()).toBeNull();
	await expect(card(page, 'Plan A login screen')).toHaveCount(1);
	await expect(card(page, 'Plan C session')).toHaveCount(1);
	await expect(card(page, 'Plan E dark theme')).toHaveCount(0);

	// Make the shown version the release version: its cards carry the Release version mark and the star marks it in the list.
	await headerMenu(page).click();
	await page.getByRole('button', { name: 'Make the shown v1.8.0 the release version' }).click();
	await expect.poll(releaseVersion).toBe('v1.8.0');
	await expect(headerMenu(page)).toHaveAccessibleName('Version filter: v1.8.0');
	await expect(card(page, 'Plan A login screen').locator('.ver-tag')).toContainText('Release version');
	await headerMenu(page).click();
	await expect(page.getByRole('radio', { name: /^v1\.8\.0/ }).locator('xpath=..')).toContainText('\u2605');
	await expect(page.getByRole('button', { name: /the release version$/ })).toHaveCount(0);
	// All clears the filter again.
	await page.getByRole('radio', { name: 'All' }).click();
	await expect(headerMenu(page)).toHaveAccessibleName('Version filter: All');
	await expect(card(page, 'Plan E dark theme')).toHaveCount(1);
	expect(await plannedVersions()).toMatchObject({ [cards.a]: 'v1.8.0', [cards.e]: 'v1.9.0' });

	// A task's planned version is edited in its edit dialog, and saved with the dialog.
	const taskDialog = await openTaskDialog(page, 'Plan B reset mail', cards.b);
	await expect(taskDialog.getByRole('radio', { name: /Make it the release version \(v1\.8\.0\)/ })).toBeEnabled();
	await taskDialog.getByRole('radio', { name: /v1\.9\.0/ }).click();
	expect((await plannedVersions())[cards.b]).toBeNull();
	await taskDialog.getByRole('button', { name: 'Save', exact: true }).click();
	await expect(taskDialog).toHaveCount(0);
	await expect.poll(plannedVersions).toMatchObject({ [cards.b]: 'v1.9.0' });

	// An epic's tasks are planned together in the epic's edit dialog: every task that is not done takes the version.
	await openHeaderItem(page, 'Epics');
	await page.getByRole('list', { name: 'Epics in this project' }).getByRole('listitem').filter({ hasText: 'Login work' }).getByRole('button').first().click();
	await page.getByRole('button', { name: 'Edit epic', exact: true }).click();
	const epicDialog = page.getByRole('dialog', { name: /Epic preview/ });
	const save = epicDialog.getByRole('button', { name: 'Save', exact: true });
	await expect(save).toBeDisabled();
	await epicDialog.getByRole('button', { name: 'Planned versions' }).click();
	await epicDialog.getByRole('button', { name: 'Planned version of the epic: No change' }).click();
	await expect(page.getByRole('radio', { name: 'No change' })).toBeChecked();
	await expect(page.getByRole('radio', { name: 'Release version (v1.8.0)' })).toBeVisible();
	await page.getByRole('radio', { name: 'v1.9.0' }).click();
	await expect(epicDialog.getByRole('button', { name: 'Planned version of the epic: v1.9.0' })).toBeVisible();
	expect(await plannedVersions()).toMatchObject({ [cards.a]: 'v1.8.0', [cards.c]: null });
	await expect(save).toBeEnabled();
	await save.click();
	await expect(epicDialog).toHaveCount(0);
	await expect
		.poll(plannedVersions)
		.toMatchObject({ [cards.a]: 'v1.9.0', [cards.b]: 'v1.9.0', [cards.c]: 'v1.9.0', [cards.d]: 'v1.9.0', [cards.e]: 'v1.9.0' });
});

test('the task dialog refuses to overwrite a change made elsewhere and shows the newer value', async ({ page }) => {
	const cards = await seed('v1.8.0');
	await page.setViewportSize({ width: 1440, height: 900 });
	await page.goto('/');
	await waitForSync(page);
	const dialog = await openTaskDialog(page, 'Plan C session', cards.c);
	await dialog.getByRole('radio', { name: /v1\.9\.0/ }).click();
	// The task is planned for another version from elsewhere (the CLI, another browser) before the save.
	await planTask(cards.c, 'v1.8.0');
	await dialog.getByRole('button', { name: 'Save', exact: true }).click();
	await expect(dialog.getByRole('alert').filter({ hasText: 'v1.8.0' })).toBeVisible();
	// Nothing was written over it, and the dialog stays open.
	expect((await plannedVersions())[cards.c]).toBe('v1.8.0');
	await expect(dialog).toBeVisible();
});

test('the release version is unavailable in the task dialog while none is set', async ({ page }) => {
	const cards = await seed(null);
	await page.setViewportSize({ width: 1440, height: 900 });
	await page.goto('/');
	await waitForSync(page);
	const dialog = await openTaskDialog(page, 'Plan C session', cards.c);
	await expect(dialog.getByRole('radio', { name: /Make it the release version/ })).toBeDisabled();
	await expect(dialog.getByText('Unavailable: no release version is set')).toBeVisible();
});

test('the phone shows the planned version, filters by it and lists a new one, but cannot change the release version', async ({ page }) => {
	const cards = await seed('v1.8.0');
	await page.setViewportSize({ width: 390, height: 844 });
	await page.goto('/');
	await page.getByRole('tab', { name: /Todo/ }).click();
	await expect(page.locator('.monitor-card', { hasText: 'Plan A login screen' }).locator('.ver-tag')).toContainText('Release version');
	// The tag sits in the date row, at the left of the time.
	const meta = page.locator('.monitor-card', { hasText: 'Plan A login screen' }).locator('.monitor-card__meta');
	await expect(meta.locator('.ver-tag')).toHaveCount(1);
	const [metaTag, metaTime] = await Promise.all([meta.locator('.ver-tag').boundingBox(), meta.locator('.monitor-card__time').boundingBox()]);
	expect(metaTag!.x + metaTag!.width).toBeLessThanOrEqual(metaTime!.x);

	// The top bar shows the logo alone, and the version filter starts on the release version.
	await expect(page.locator('.header__app').getByText('Agent Project')).toHaveClass(/visually-hidden/);
	await expect(page.locator('.header__logo img')).toBeVisible();
	const filter = page.getByRole('button', { name: /^Version filter:/ });
	await expect(filter).toHaveAccessibleName('Version filter: v1.8.0');
	await expect(page.locator('.monitor-card', { hasText: 'Plan E dark theme' })).toHaveCount(0);
	await expect(page.locator('.monitor-card', { hasText: 'Plan C session' })).toHaveCount(1);
	const bar = await page.locator('.monitor-shell > .header').boundingBox();
	const button = await filter.boundingBox();
	expect(button!.x + button!.width).toBeLessThanOrEqual(bar!.x + bar!.width);

	// The sheet lists a new version without changing the filter or the release version, and has no release button.
	await filter.click();
	await page.getByLabel('New version').fill('v2.0.0');
	await page.getByRole('button', { name: 'Add this version to the project' }).click();
	await expect(page.getByRole('radio', { name: /^v2\.0\.0/ })).toBeVisible();
	await expect(filter).toHaveAccessibleName('Version filter: v1.8.0');
	expect(await releaseVersion()).toBe('v1.8.0');
	await expect(page.getByRole('radio', { name: /^v1\.8\.0/ })).toContainText('\u2605');
	await expect(page.getByRole('button', { name: /the release version$/ })).toHaveCount(0);

	// All shows every version's cards again; the Backlog is never filtered.
	await page.getByRole('radio', { name: 'All' }).click();
	await expect(filter).toHaveAccessibleName('Version filter: All');
	await expect(page.locator('.monitor-card', { hasText: 'Plan E dark theme' })).toHaveCount(1);
	expect(cards.e).toBeGreaterThan(0);
	expect(await releaseVersion()).toBe('v1.8.0');
	await page.locator('.monitor-card', { hasText: 'Plan A login screen' }).getByRole('button', { name: 'Plan A login screen' }).click();
	await expect(page.getByText('Planned version')).toHaveCount(0);

	// Nothing overflows sideways, at 320px too.
	for (const width of [390, 320]) {
		await page.setViewportSize({ width, height: 844 });
		expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
	}
});
