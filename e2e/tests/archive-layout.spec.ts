import { expect, test } from "../helpers/fixtures";
import { writeFileSync } from "node:fs";
import { mainProject, resetBoard } from "../helpers/agpj";
import { resetPreferences, setPreferences } from "../helpers/preferences";

test.beforeEach(() => resetPreferences());
test.afterEach(() => resetPreferences());

for (const mode of ["light", "dark"] as const) {
	for (const locale of ["en", "ja"] as const) {
		for (const width of [960, 1280, 2048]) {
			test(`archive layout and selection at ${width}px, ${locale}, ${mode}`, async ({
				page,
			}, testInfo) => {
				resetBoard(mainProject);
				await page.setViewportSize({ width, height: 900 });
				await setPreferences({ locale, theme: { mode, accent: "blue" } });

				// Keep layout checks independent of writes; all cards are temporary response fixtures.
				// The board is drawn from the Summary, so that is the response replaced.
				await page.route("**/api/projects/*/summary", async (route) => {
					const response = await route.fetch();
					const summary = await response.json();
					const task = summary.tasks.find(
						(item: { taskStatus: string }) => item.taskStatus === "done",
					);
					summary.tasks = Array.from({ length: 24 }, (_, index) => ({
						...task,
						id: index + 1,
						title: `Archive layout task ${index + 1}: 日本語の長いタイトルと対象タスクの確認`,
					}));
					await route.fulfill({ json: summary });
				});
				await page.route("**/api/projects/*/archive?*", async (route) => {
					// The archive lists whole cards with their conditions; the Summary only counts them, so the
					// conditions are rebuilt from the counts.
					const summaryUrl = route
						.request()
						.url()
						.replace(/\/archive\?.*$/, "/summary");
					const summary = await (await page.request.get(summaryUrl)).json();
					await route.fulfill({
						json: {
							...summary,
							total: 3,
							nextCursor: null,
							tasks: summary.tasks.slice(0, 3).map(
								(task: { acceptance: { done: number; total: number } }) => ({
									...task,
									archivedAt: "2026-10-08T00:00:00Z",
									attachments: [],
									acceptance: Array.from({ length: task.acceptance.total }, (_, index) => ({
										id: String(index + 1),
										text: "",
										done: index < task.acceptance.done,
									})),
								}),
							),
						},
					});
				});
				await page.goto("/");
				await expect(page.locator(".lane-header__archive")).toBeEnabled();
				await page.locator(".header__archive").click();
				const archive = page.locator(".list-pane");
				const list = page.locator(".list-pane__rows");
				const row = page.locator(".archive-row > button").first();
				await expect(row).toBeVisible();
				const bodyBounds = (await page.locator(".app__body").boundingBox())!;
				const archiveBounds = (await archive.boundingBox())!;
				const rowBounds = (await row.boundingBox())!;
				const date = row.locator(".archive-row__date");
				expect(archiveBounds.width).toBeLessThan(bodyBounds.width);
				expect(archiveBounds.x + archiveBounds.width).toBeLessThanOrEqual(
					bodyBounds.width,
				);
				const listPadding = 48;
				expect(rowBounds.width).toBeCloseTo(
					archiveBounds.width - listPadding,
					0,
				);
				for (const selector of [
					".app__body",
					".list-pane",
					".list-pane__rows",
					".archive-row",
					".archive-row > button",
					".detail",
				]) {
					const pane = page.locator(selector).first();
					await expect(pane).toBeVisible();
					const { clientWidth, scrollWidth } = await pane.evaluate(
						(element) => ({
							clientWidth: element.clientWidth,
							scrollWidth: element.scrollWidth,
						}),
					);
					expect(
						scrollWidth,
						`${selector} must not overflow horizontally at ${width}px`,
					).toBeLessThanOrEqual(clientWidth);
				}
				const dateLineCount = await date.evaluate((element) => {
					const range = document.createRange();
					range.selectNodeContents(element);
					return range.getClientRects().length;
				});
				expect(
					dateLineCount,
					`archive date must stay on one line at ${width}px`,
				).toBe(1);
				await page.screenshot({
					path: testInfo.outputPath("01-archive-list.png"),
				});
				if (width === 1280 && locale === "ja" && mode === "light") {
					const detailBounds = (await page.locator(".detail").boundingBox())!;
					writeFileSync(
						testInfo.outputPath("archive-geometry.json"),
						JSON.stringify(
							{
								viewport: { width, height: 900 },
								url: page.url(),
								body: bodyBounds,
								archivePane: archiveBounds,
								row: rowBounds,
								detail: detailBounds,
							},
							null,
							2,
						),
					);
				}

				await row.click();
				const detail = page.locator(".detail");
				await expect(detail).toBeVisible();
				const detailBounds = (await detail.boundingBox())!;
				const listBounds = (await list.boundingBox())!;
				expect(detailBounds.x + detailBounds.width).toBeLessThanOrEqual(width);
				expect(detailBounds.x).toBeGreaterThan(listBounds.x + listBounds.width);
				expect(
					await page.evaluate(() => document.documentElement.scrollWidth),
				).toBeLessThanOrEqual(width);
				await page.screenshot({
					path: testInfo.outputPath("02-archive-detail.png"),
				});
				await detail.locator(".detail__close").click();
				await expect(detail.locator(".detail__empty-message")).toBeVisible();
				expect((await row.boundingBox())!.width).toBeCloseTo(
					rowBounds.width,
					0,
				);
				if (width === 1280 && locale === "ja" && mode === "light")
					await page.screenshot({
						path: testInfo.outputPath("03-archive-closed.png"),
					});
				await page.locator(".header__back").click();

				// The Done lane's archive button starts the selection with every done card picked.
				const trigger = page.locator(".lane-header__archive");
				await trigger.click();
				const bar = page.getByRole("region", {
					name:
						locale === "ja"
							? "選んだ完了タスクをアーカイブする"
							: "Archive the picked completed tasks",
				});
				await expect(bar).toBeVisible();
				await expect(bar.getByRole("status")).toHaveText(
					locale === "ja" ? "24 枚を選択中" : "24 selected",
				);
				const laneBounds = (await page
					.locator('.lane[data-column="done"]')
					.boundingBox())!;
				const barBounds = (await bar.boundingBox())!;
				expect(barBounds.x).toBeGreaterThanOrEqual(laneBounds.x - 1);
				expect(barBounds.x + barBounds.width).toBeLessThanOrEqual(
					laneBounds.x + laneBounds.width + 1,
				);
				const cancel = bar.getByRole("button", {
					name: locale === "ja" ? "やめる" : "Cancel",
					exact: true,
				});
				const submit = bar.getByRole("button", {
					name: /archive \(24\)|アーカイブ \(24\)/i,
				});
				// The two buttons are the same size; the fill and the outline tell them apart.
				const [cancelBox, submitBox] = [
					(await cancel.boundingBox())!,
					(await submit.boundingBox())!,
				];
				expect(Math.abs(cancelBox.width - submitBox.width)).toBeLessThan(2);
				expect(Math.abs(cancelBox.height - submitBox.height)).toBeLessThan(2);
				if (width === 1280)
					await page.screenshot({
						path: testInfo.outputPath("03-archive-selection.png"),
					});
				await cancel.click();
				await expect(bar).toHaveCount(0);
			});
		}
	}
}

test("archive empty state matches the Pen guidance and shared detail placeholder", async ({
	page,
}, testInfo) => {
	resetBoard(mainProject);
	await page.setViewportSize({ width: 1280, height: 900 });
	await setPreferences({ locale: "ja", theme: { mode: "light", accent: "blue" } });
	await page.route("**/api/projects/*/archive?*", async (route) => {
		const response = await route.fetch();
		await route.fulfill({
			response,
			json: { ...(await response.json()), tasks: [], total: 0, nextCursor: null },
		});
	});
	await page.goto("/");
	await expect(page.getByRole("region", { name: "未着手" })).toBeVisible();
	await page.locator(".header__archive").click();
	await expect(page.locator(".list-pane__count")).toHaveText("0件");
	await expect(page.locator(".list-pane__rows")).toHaveCount(0);
	await expect(page.locator(".list-pane__empty svg")).toBeVisible();
	await expect(
		page.getByText("アーカイブされたタスクはありません"),
	).toBeVisible();
	await expect(
		page.getByText("タスクをアーカイブすると、ここに表示されます。"),
	).toBeVisible();
	await expect(page.locator(".detail__empty-message")).toBeVisible();
	await page.screenshot({ path: testInfo.outputPath("04-archive-empty.png") });
});
