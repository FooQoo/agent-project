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
 * Resolves to true when the board can render. A page that reloads to be controlled, or a browser
 * with no Service Worker, resolves to false; the latter shows a short notice in `root`.
 */
export async function startMock(mode: 'mock' | 'demo', root: HTMLElement): Promise<boolean> {
  const locale = mode === 'demo' ? demoLocale(browserLanguages(navigator)) : undefined;
  if (locale) applyLocale(locale);
  const server = createMockServer({ fixture, longMarkdown, locale, demo: mode === 'demo' });
  const state = await startMockWorker(server);
  if (state === 'unsupported') showUnsupported(root, locale ?? 'en');
  return state === 'ready';
}

const NOTICE = {
  en: 'This demo needs Service Workers, which this browser or window does not offer. Open the page in another browser, or outside a private window.',
  ja: 'このデモにはService Workerが必要ですが、このブラウザまたはウィンドウでは使えません。別のブラウザか、プライベートウィンドウの外で開いてください。',
} as const;

function showUnsupported(root: HTMLElement, locale: 'en' | 'ja'): void {
  const notice = document.createElement('p');
  notice.setAttribute('role', 'alert');
  notice.style.cssText = 'max-width:36rem;margin:3rem auto;padding:0 1rem;font:1rem/1.6 sans-serif';
  notice.textContent = NOTICE[locale];
  root.replaceChildren(notice);
}
