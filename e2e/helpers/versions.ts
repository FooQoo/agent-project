import { readFileSync, utimesSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { mainHome, mainOrigin, mainProject, projectId, workspace } from './agboard';

/** The write token the server puts in the page it serves. */
async function pageToken(origin: string): Promise<string> {
	const html = await (await fetch(`${origin}/`)).text();
	const token = html.match(/<meta name="agboard-token" content="([^"]*)"/)?.[1];
	if (!token) throw new Error(`no write token in the page at ${origin}`);
	return token;
}

async function send(method: 'POST' | 'PUT', path: string, body: unknown): Promise<unknown> {
	const origin = mainOrigin();
	const response = await fetch(`${origin}/api/projects/${projectId(mainProject, mainHome())}${path}`, {
		method,
		headers: {
			'Content-Type': 'application/json',
			Origin: origin,
			'Sec-Fetch-Site': 'same-origin',
			'X-Agboard-Token': await pageToken(origin),
		},
		body: JSON.stringify(body),
	});
	if (!response.ok) throw new Error(`${method} ${path} answered ${response.status}: ${await response.text()}`);
	return response.json();
}

/** Lists versions and sets the current one, as the version settings do. */
export async function seedVersions(versions: string[], current: string | null): Promise<void> {
	for (const name of versions) await send('POST', '/versions', { name });
	await send('PUT', '/versions/current', { version: current });
}

/** Plans one task for a version, as the task detail's picker does. */
export async function planTask(id: number, version: string | null): Promise<void> {
	await send('PUT', `/tasks/${id}/planned-version`, version === null ? { choice: 'unset' } : { choice: 'named', version });
}

/** The planned version of every card, read from the Summary the board reads. */
export async function plannedVersions(): Promise<Record<number, string | null>> {
	const response = await fetch(`${mainOrigin()}/api/projects/${projectId(mainProject, mainHome())}/summary`);
	const summary = (await response.json()) as { tasks: { id: number; plannedVersion: string | null }[] };
	return Object.fromEntries(summary.tasks.map((task) => [task.id, task.plannedVersion]));
}

/** The project's current version, as the Summary names it. */
export async function currentVersion(): Promise<string | null> {
	const response = await fetch(`${mainOrigin()}/api/projects/${projectId(mainProject, mainHome())}/summary`);
	return ((await response.json()) as { project: { currentVersion: string | null } }).project.currentVersion;
}

/**
 * Takes the versions and the current version out of the main project, so a spec that listed some leaves the
 * board as the others expect it: a project that lists a version shows a planned-version tag on every card.
 * Cards lose their planned version when the board is reset to its fixture.
 */
export function clearVersions(): void {
	const file = path.join(workspace(mainProject, mainHome()).folder, 'project.json');
	const project = JSON.parse(readFileSync(file, 'utf8')) as Record<string, unknown>;
	delete project.versions;
	delete project.currentVersion;
	writeFileSync(file, `${JSON.stringify(project, null, 2)}\n`);
	// A serve that polls compares modification times in whole seconds: make this write a later one.
	const later = new Date(Date.now() + 1000);
	utimesSync(file, later, later);
}
