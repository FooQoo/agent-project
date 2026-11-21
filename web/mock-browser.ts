import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';

const SEED_ID = 'virtual:agboard-mock-seed';
const RESOLVED_SEED_ID = `\0${SEED_ID}`;
const FIXTURE_PATH = resolve(import.meta.dirname, 'test/fixtures/board.json');
const LONG_MARKDOWN_PATH = resolve(import.meta.dirname, 'test/fixtures/long-markdown.md');
const WORKER_FILE = 'mockServiceWorker.js';

/** The worker script of the installed `msw`: it checks its own version, so it is never copied. */
function workerSource(): string {
  const entry = fileURLToPath(import.meta.resolve('msw'));
  return readFileSync(resolve(dirname(entry), '..', WORKER_FILE), 'utf8');
}

export interface MockBrowserOptions {
  /** The token the page sends; the sample-data server accepts any. */
  token: string;
  /** Empty the build ID, so the page never reloads for a newer build (the demo). */
  noBuildId?: boolean;
}

/**
 * The sample-data server runs in the page, behind a Service Worker, in the dev sample-data mode
 * (`vite --mode mock`) and in the demo build (`vite build --mode demo`). This plugin gives the page
 * the sample data as `virtual:agboard-mock-seed`, serves the MSW worker script beside the page
 * (the dev server) or emits it into the build (the demo, next to `index.html`, so its scope is the
 * demo folder), and fills the page's server placeholders. Only registered in those two modes.
 */
export function mockBrowser(options: MockBrowserOptions): Plugin {
  return {
    name: 'agboard-mock-browser',
    resolveId: (id) => (id === SEED_ID ? RESOLVED_SEED_ID : null),
    load(id) {
      if (id !== RESOLVED_SEED_ID) return null;
      const fixture = readFileSync(FIXTURE_PATH, 'utf8');
      const longMarkdown = readFileSync(LONG_MARKDOWN_PATH, 'utf8');
      return `export const fixture = ${fixture};\nexport const longMarkdown = ${JSON.stringify(longMarkdown)};\n`;
    },
    configureServer(vite) {
      vite.middlewares.use(`/${WORKER_FILE}`, (_req, res) => {
        res.setHeader('Content-Type', 'text/javascript');
        res.setHeader('Cache-Control', 'no-store');
        res.end(workerSource());
      });
    },
    generateBundle() {
      this.emitFile({ type: 'asset', fileName: WORKER_FILE, source: workerSource() });
    },
    transformIndexHtml(html) {
      const page = html.replace('{{AGBOARD_TOKEN}}', options.token);
      return options.noBuildId ? page.replace('{{AGBOARD_BUILD_ID}}', '') : page;
    },
  };
}
