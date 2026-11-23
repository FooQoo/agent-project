import { cleanup, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { Header, type ViewMode } from './Header';

const NAMES = ['Docs', 'Epics', 'Archive', 'Settings'];

async function openMenu(props: { viewMode: ViewMode; canAdd?: boolean; switchDisabled?: boolean }) {
  const handlers = { onDocs: vi.fn(), onEpics: vi.fn(), onArchive: vi.fn() };
  render(<Header menu settings={{ render: () => null }} {...handlers} {...props} />);
  await userEvent.click(screen.getByRole('button', { name: /^menu$/i }));
  const menu = within(await screen.findByRole('menu'));
  return { menu, handlers };
}

describe('Header menu', () => {
  it.each<ViewMode>(['board', 'epics', 'archive'])(
    'lists the same four items in order on the %s screen',
    async (viewMode) => {
      const { menu } = await openMenu({ viewMode, canAdd: viewMode === 'board' });
      expect(menu.getAllByRole('menuitem').map((item) => item.textContent)).toEqual(NAMES);
    },
  );

  it('marks the open screen as current and not selectable', async () => {
    for (const [viewMode, name] of [
      ['epics', 'Epics'],
      ['archive', 'Archive'],
    ] as const) {
      const { menu } = await openMenu({ viewMode });
      const item = menu.getByRole('menuitem', { name });
      expect(item.getAttribute('aria-current')).toBe('page');
      expect(item.getAttribute('aria-disabled')).toBe('true');
      expect(
        menu.getAllByRole('menuitem').filter((i) => i.hasAttribute('aria-current')),
      ).toHaveLength(1);
      cleanup();
    }
  });

  it('navigates from the Epics screen to the other screens', async () => {
    const { menu, handlers } = await openMenu({ viewMode: 'epics' });
    await userEvent.click(menu.getByRole('menuitem', { name: 'Docs' }));
    expect(handlers.onDocs).toHaveBeenCalledTimes(1);
    expect(handlers.onEpics).not.toHaveBeenCalled();
  });

  it('has no current item on the board and enables the destinations once loaded', async () => {
    const { menu } = await openMenu({ viewMode: 'board', canAdd: true });
    for (const item of menu.getAllByRole('menuitem')) {
      expect(item.hasAttribute('aria-current')).toBe(false);
      expect(item.getAttribute('aria-disabled')).not.toBe('true');
    }
  });

  it('disables the destinations, not Settings, while a form is open or the board is loading', async () => {
    for (const props of [
      { viewMode: 'board' as const, canAdd: true, switchDisabled: true },
      { viewMode: 'board' as const, canAdd: false },
      { viewMode: 'archive' as const, switchDisabled: true },
    ]) {
      const { menu } = await openMenu(props);
      for (const name of ['Docs', 'Epics', 'Archive']) {
        expect(menu.getByRole('menuitem', { name }).getAttribute('aria-disabled')).toBe('true');
      }
      expect(
        menu.getByRole('menuitem', { name: 'Settings' }).getAttribute('aria-disabled'),
      ).not.toBe('true');
      cleanup();
    }
  });
});
