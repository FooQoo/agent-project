import { fixture, longMarkdown } from 'virtual:agboard-mock-seed';
import { applyLocale } from '../../../shared/i18n';
import { createMockServer } from '../mock-server/server.ts';
import { startMockWorker } from './browser.ts';
import { browserLanguages, demoLocale } from './locale.ts';

/**
 * Starts the board's sample-data server in the page, behind a Service Worker, for the two modes
 * that run without `agboard serve`: the dev sample-data mode (`vite --mode mock`) and the browser
 * demo (`vite build --mode demo`). `app/entry/board.tsx` loads this only in those modes, so the
 * product build contains none of it. Nothing is saved: a reload starts again from the sample data.
 *
 * The demo starts in the browser's language (`demoLocale`) before the first screen is drawn, in
 * place of any language an earlier visit left in the browser, and opens with the first-run guide.
 * The dev mode keeps three sample projects in English.
 *
 * Resolves to true when the board can render. A page that reloads to be controlled resolves to
 * false. So do a browser with no Service Worker and a worker that fails to start; both show a
 * short notice in `root`, and the board stays unrendered.
 */
export async function startMock(mode: 'mock' | 'demo', root: HTMLElement): Promise<boolean> {
  const locale = mode === 'demo' ? demoLocale(browserLanguages(navigator)) : undefined;
  if (locale) applyLocale(locale);
  const server = createMockServer({ fixture, longMarkdown, locale, demo: mode === 'demo' });
  try {
    const state = await startMockWorker(server);
    if (state === 'unsupported') showNotice(root, locale ?? 'en', 'unsupported');
    return state === 'ready';
  } catch {
    // The browser refused the worker script or its registration.
    showNotice(root, locale ?? 'en', 'failed');
    return false;
  }
}

const NOTICES = {
  unsupported: {
    en: 'This demo needs Service Workers, which this browser or window does not offer. Open the page in another browser, or outside a private window.',
    ja: 'このデモにはService Workerが必要ですが、このブラウザまたはウィンドウでは使えません。別のブラウザか、プライベートウィンドウの外で開いてください。',
  },
  failed: {
    en: 'The demo could not start: the browser did not accept its Service Worker. Reload the page, or open it in another browser.',
    ja: 'デモを開始できませんでした。ブラウザがService Workerを受け付けませんでした。ページを再読み込みするか、別のブラウザで開いてください。',
  },
} as const;

function showNotice(root: HTMLElement, locale: 'en' | 'ja', kind: keyof typeof NOTICES): void {
  const notice = document.createElement('p');
  notice.setAttribute('role', 'alert');
  notice.style.cssText = 'max-width:36rem;margin:3rem auto;padding:0 1rem;font:1rem/1.6 sans-serif';
  notice.textContent = NOTICES[kind][locale];
  root.replaceChildren(notice);
}
