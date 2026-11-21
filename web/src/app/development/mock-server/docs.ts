import { SAMPLE_IMAGE } from './boards.ts';
import type { Routes } from './types.ts';
import { error, json, noContent, projectNotFound } from './wire.ts';

// The docs viewer's routes: a small sample tree of files, so the dialog opens in the demo, in
// Storybook and in the tests. The files are the same for every project and never change.

const FILES: Record<string, string> = {
  'README.md': '# Sample docs\n\nThese files come from the sample-data server.\n',
  'guide/start.md':
    '# Getting started\n\nOpen a card to read its detail.\n\n![Sample](../sample.svg)\n',
};

const TREE = {
  nodes: [
    { name: 'README.md', path: 'README.md', kind: 'file', viewer: 'markdown' },
    {
      name: 'guide',
      path: 'guide',
      kind: 'dir',
      children: [{ name: 'start.md', path: 'guide/start.md', kind: 'file', viewer: 'markdown' }],
    },
    { name: 'sample.svg', path: 'sample.svg', kind: 'file', viewer: 'image' },
  ],
  truncated: false,
};

const DOCS_PATH = /^\/api\/projects\/([^/]+)\/docs\/(tree|file|assets|open)$/;

/** `docs/tree`, `docs/file`, `docs/assets` and `docs/open` of a project. */
export const docsRoutes: Routes = (context, request) => {
  const match = DOCS_PATH.exec(request.url.pathname);
  if (!match) return null;
  if (!context.boards()[match[1] ?? '']) return projectNotFound();
  const path = request.url.searchParams.get('path') ?? '';
  switch (match[2]) {
    case 'tree':
      return request.method === 'GET' ? json(200, TREE) : null;
    case 'file': {
      const content = FILES[path];
      if (request.method !== 'GET') return null;
      return content === undefined
        ? error(404, 'not_found', 'not found.', 'No such file in the docs.')
        : json(200, { path, content, revision: `mock-${content.length}` });
    }
    case 'assets':
      return request.method === 'GET'
        ? {
            kind: 'response',
            status: 200,
            headers: { 'Content-Type': 'image/svg+xml' },
            body: SAMPLE_IMAGE,
          }
        : null;
    default:
      return request.method === 'POST' ? noContent() : null;
  }
};
