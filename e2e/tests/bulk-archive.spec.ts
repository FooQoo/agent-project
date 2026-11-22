import { expect, test } from "../helpers/fixtures";
import { listTasks, mainProject, resetBoard } from "../helpers/agpj";
import { resetPreferences } from "../helpers/preferences";

// #686: the Done lane's archive is a selection with every done card picked. These specs press it through to
// the saved result on the PC board (a bar under the lane) and on the phone screen (a short bottom drawer).
test.beforeEach(() => resetPreferences());
test.afterEach(() => resetPreferences());

const doneIds = () =>
	listTasks(mainProject)
		.filter((task) => task.taskStatus === "done" && !(task as { archivedAt?: string | null }).archivedAt)
		.map((task) => task.id);

const BAR = "Archive the picked completed tasks";

test("the PC board's archive button picks every done card, and archives only the picked ones", async ({
	page,
}) => {
	resetBoard(mainProject);
	const targets = doneIds();
	expect(targets.length).toBeGreaterThanOrEqual(2);
	const [kept, ...archived] = targets;
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto("/");
	const trigger = page.locator(".lane-header__archive");
	await expect(trigger).toBeEnabled();
	await trigger.click();

	// Every done card starts picked, and the bar counts them.
	const bar = page.getByRole("region", { name: BAR });
	await expect(bar).toBeVisible();
	await expect(bar.getByRole("status")).toHaveText(`${targets.length} selected`);
	for (const id of targets) await expect(page.getByRole("checkbox", { name: `Select #${id}` })).toBeChecked();

	// Cancel changes nothing, and pressing the button again picks every card again.
	await bar.getByRole("button", { name: "Cancel" }).click();
	await expect(bar).toHaveCount(0);
	expect(doneIds()).toEqual(targets);
	await trigger.click();
	await expect(bar.getByRole("status")).toHaveText(`${targets.length} selected`);

	// The two buttons are the same size.
	const [cancel, submit] = [
		(await bar.getByRole("button", { name: "Cancel" }).boundingBox())!,
		(await bar.getByRole("button", { name: `Archive (${targets.length})` }).boundingBox())!,
	];
	expect(Math.abs(cancel.width - submit.width)).toBeLessThan(2);
	expect(Math.abs(cancel.height - submit.height)).toBeLessThan(2);

	// A click on a card unpicks it; only the picked cards are archived.
	await page.getByRole("checkbox", { name: `Select #${kept}` }).click();
	await expect(bar.getByRole("status")).toHaveText(`${archived.length} selected`);
	await bar.getByRole("button", { name: `Archive (${archived.length})` }).click();
	await expect(bar).toHaveCount(0);
	await expect.poll(doneIds).toEqual([kept]);
});

test("the PC board's archive button cannot archive with nothing picked", async ({ page }) => {
	resetBoard(mainProject);
	const targets = doneIds();
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto("/");
	await page.locator(".lane-header__archive").click();
	const bar = page.getByRole("region", { name: BAR });
	for (const id of targets) await page.getByRole("checkbox", { name: `Select #${id}` }).click();
	await expect(bar.getByRole("button", { name: "Archive (0)" })).toBeDisabled();
	expect(doneIds()).toEqual(targets);
});

// Below 768px the same board shows the phone screen. Its button, pressed in a narrow desktop window, opens
// the selection drawer with every done card picked.
for (const width of [767, 390, 320]) {
	test(`the phone screen's archive selection archives only the picked done cards in a ${width}px window`, async ({
		page,
	}) => {
		resetBoard(mainProject);
		const targets = doneIds();
		expect(targets.length).toBeGreaterThanOrEqual(2);
		const [kept, ...archived] = targets;
		await page.setViewportSize({ width, height: 800 });
		await page.goto("/");
		await expect(page.getByRole("tablist", { name: "Lanes" })).toBeVisible();
		await page.getByRole("tab", { name: /^Done / }).click();
		const trigger = page.getByRole("button", { name: "Select and archive" });
		await expect(trigger).toBeEnabled();
		await trigger.click();

		const drawer = page.getByRole("region", { name: BAR });
		await expect(drawer.getByRole("status")).toHaveText(`${targets.length} selected`);
		// The drawer does not dim the cards, and the buttons are the same size and inside the window.
		await expect(page.locator("[data-slot='drawer-overlay']")).toHaveCount(0);
		await expect
			.poll(async () => {
				const [button, dock] = await Promise.all([
					drawer.getByRole("button", { name: `Archive (${targets.length})` }).boundingBox(),
					page.locator(".phone-nav").boundingBox(),
				]);
				return (button?.y ?? 9999) + (button?.height ?? 0) <= (dock?.y ?? 0);
			})
			.toBe(true);
		const [cancel, submit] = [
			(await drawer.getByRole("button", { name: "Cancel" }).boundingBox())!,
			(await drawer.getByRole("button", { name: `Archive (${targets.length})` }).boundingBox())!,
		];
		expect(Math.abs(cancel.width - submit.width)).toBeLessThan(2);
		expect(Math.abs(cancel.height - submit.height)).toBeLessThan(2);
		expect(cancel.x).toBeGreaterThanOrEqual(0);
		expect(submit.x + submit.width).toBeLessThanOrEqual(width);

		// Escape (pressed inside the drawer) ends the mode and changes nothing.
		await drawer.getByRole("button", { name: "Cancel" }).focus();
		await page.keyboard.press("Escape");
		await expect(drawer).toHaveCount(0);
		expect(doneIds()).toEqual(targets);

		// Pick again, unpick one card, and archive the rest.
		await trigger.click();
		await page.getByRole("checkbox", { name: `Select #${kept}` }).click();
		await expect(drawer.getByRole("status")).toHaveText(`${archived.length} selected`);
		await drawer.getByRole("button", { name: `Archive (${archived.length})` }).click();
		await expect(drawer).toHaveCount(0);
		await expect.poll(doneIds).toEqual([kept]);
		await expect(page.getByRole("tab", { name: "Done 1" })).toBeVisible();
	});
}
