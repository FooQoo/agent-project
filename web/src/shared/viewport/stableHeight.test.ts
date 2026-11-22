import { afterEach, describe, expect, it, vi } from 'vitest';
import { nextStableHeight, revealFocusedField, watchStableHeight } from './stableHeight';

describe('nextStableHeight', () => {
  it('keeps the height while typing shrinks the viewport', () => {
    const kept = { width: 1000, height: 800 };
    expect(nextStableHeight(kept, { width: 1000, height: 500 }, true)).toBe(kept);
    expect(nextStableHeight(kept, { width: 1000, height: 500 }, false)).toEqual({
      width: 1000,
      height: 500,
    });
  });
});

describe('watchStableHeight', () => {
  const root = document.documentElement;
  afterEach(() => {
    root.removeAttribute('data-keyboard');
    document.body.replaceChildren();
  });

  it('marks the keyboard only while a text field has the focus', () => {
    const target = Object.assign(new EventTarget(), {
      document,
      innerWidth: 1000,
      innerHeight: 800,
    }) as unknown as Window & { innerHeight: number };
    const field = document.body.appendChild(document.createElement('input'));
    const stop = watchStableHeight(target);
    field.focus();
    target.innerHeight = 500;
    target.dispatchEvent(new Event('resize'));
    expect(root.hasAttribute('data-keyboard')).toBe(true);
    expect(root.style.getPropertyValue('--stable-vh')).toBe('800px');
    // The focus leaves before the keyboard has finished closing: dialogs are centered again at once.
    field.blur();
    expect(root.hasAttribute('data-keyboard')).toBe(false);
    stop();
  });
});

describe('revealFocusedField', () => {
  afterEach(() => {
    document.body.replaceChildren();
    vi.restoreAllMocks();
  });

  it.each(['dialog-content', 'drawer-popup'])(
    'scrolls the focused field of a %s into sight',
    async (slot) => {
      const surface = document.body.appendChild(document.createElement('div'));
      surface.dataset.slot = slot;
      const field = surface.appendChild(document.createElement('textarea'));
      const scroll = vi.fn();
      field.scrollIntoView = scroll;
      field.focus();
      revealFocusedField(window);
      await new Promise((resolve) => setTimeout(resolve, 400));
      expect(scroll).toHaveBeenCalledWith({ block: 'nearest' });
    },
  );

  it('leaves a field outside a dialog or a sheet alone', async () => {
    const field = document.body.appendChild(document.createElement('textarea'));
    const scroll = vi.fn();
    field.scrollIntoView = scroll;
    field.focus();
    revealFocusedField(window);
    await new Promise((resolve) => setTimeout(resolve, 400));
    expect(scroll).not.toHaveBeenCalled();
  });
});
