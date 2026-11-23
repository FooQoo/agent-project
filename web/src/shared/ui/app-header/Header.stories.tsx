import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, waitFor, within } from 'storybook/test';
import type { Project, ProjectId } from '../../api/contracts';
import { makeProject } from '../../../../.storybook/support/fixtures';
import { Button } from '../button';
import { Dialog, DialogContent, DialogTitle } from '../dialog';
import { Header, type HeaderSettings, type ViewMode } from './Header';

const PROJECTS: Project[] = [
  makeProject({ id: 'p-alpha' as ProjectId, name: 'alpha' }),
  makeProject({ id: 'p-beta' as ProjectId, name: 'beta' }),
];

/**
 * The settings slot the page fills. A Story cannot import the product's settings dialog, because the
 * layer rules keep shared code from importing features. This stand-in uses the same render contract:
 * it opens and closes with the header and gives the focus back to the settings button.
 */
const settingsStandIn: HeaderSettings = {
  render: ({ open, onOpenChange, restoreFocusRef }) => (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          restoreFocusRef.current?.focus();
        }}
      >
        <DialogTitle>Settings sample</DialogTitle>
      </DialogContent>
    </Dialog>
  ),
};

interface HeaderDemoProps {
  projects: Project[] | null;
  activeProjectId: ProjectId | null;
  switchDisabled?: boolean;
  canAdd?: boolean;
  viewMode?: ViewMode;
  withSettings?: boolean;
  menu?: boolean;
  onAdd?: () => void;
  onArchive?: () => void;
  onEpics?: () => void;
  onDocs?: () => void;
  onBack?: () => void;
}

/**
 * The header with the page's slots filled in. The project control is a plain button here: the real
 * switcher is covered by its own Stories under features/projects, which the header only hosts.
 */
function HeaderDemo({
  projects,
  activeProjectId,
  switchDisabled = false,
  ...props
}: HeaderDemoProps) {
  const active = projects?.find((project) => project.id === activeProjectId);
  return (
    <Header
      menu
      {...props}
      projects={projects}
      activeProjectId={activeProjectId}
      switchDisabled={switchDisabled}
      projectSwitcher={
        active ? (
          <Button variant="ghost" size="s" disabled={switchDisabled}>
            {active.name}
          </Button>
        ) : null
      }
      settings={settingsStandIn}
    />
  );
}

const meta = {
  title: 'Shared/UI/Header',
  component: HeaderDemo,
  args: {
    projects: PROJECTS,
    activeProjectId: 'p-alpha' as ProjectId,
    canAdd: true,
    viewMode: 'board',
    onAdd: fn(),
    onArchive: fn(),
    onEpics: fn(),
    onDocs: fn(),
    onBack: fn(),
  },
} satisfies Meta<typeof HeaderDemo>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Opens the header's menu button and returns the menu. */
async function openMenu(canvasElement: HTMLElement) {
  await userEvent.click(within(canvasElement).getByRole('button', { name: /^(menu|メニュー)$/i }));
  return within(await within(document.body).findByRole('menu'));
}

/** The board header: the app name, the project control and Ask AI stay; the rest is in the menu. */
export const Board: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('heading', { name: 'Agent Project' })).toBeInTheDocument();
    await expect(canvas.getByRole('button', { name: 'alpha' })).toBeEnabled();
    await expect(canvas.getByRole('button', { name: /ask ai/i })).toBeEnabled();
    await expect(canvas.queryByRole('button', { name: /docs/i })).toBeNull();
    await expect(canvas.queryByRole('button', { name: 'Settings' })).toBeNull();
    const menu = await openMenu(canvasElement);
    await expect(menu.getByRole('menuitem', { name: /docs/i })).toBeEnabled();
    await expect(menu.getByRole('menuitem', { name: /epics/i })).toBeEnabled();
    await expect(menu.getByRole('menuitem', { name: /archive/i })).toBeEnabled();
    await expect(menu.getByRole('menuitem', { name: 'Settings' })).toBeEnabled();
  },
};

/** Before the board has loaded, Ask AI, docs, epics and archive are not offered; settings stays in the menu. */
export const BoardNotReady: Story = {
  args: { canAdd: false },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole('button', { name: /ask ai/i })).toBeNull();
    const menu = await openMenu(canvasElement);
    await expect(menu.queryByRole('menuitem', { name: /archive/i })).toBeNull();
    await expect(menu.queryByRole('menuitem', { name: /docs/i })).toBeNull();
    await expect(menu.getByRole('menuitem', { name: 'Settings' })).toBeInTheDocument();
  },
};

/** While a card form or confirmation is open, the project control and the header buttons are disabled. */
export const SwitchDisabled: Story = {
  args: { switchDisabled: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('button', { name: 'alpha' })).toBeDisabled();
    await expect(canvas.getByRole('button', { name: /ask ai/i })).toBeDisabled();
    const menu = await openMenu(canvasElement);
    await expect(menu.getByRole('menuitem', { name: /docs/i })).toHaveAttribute(
      'aria-disabled',
      'true',
    );
    await expect(menu.getByRole('menuitem', { name: /epics/i })).toHaveAttribute(
      'aria-disabled',
      'true',
    );
    await expect(menu.getByRole('menuitem', { name: /archive/i })).toHaveAttribute(
      'aria-disabled',
      'true',
    );
  },
};

/** The archive screen replaces the board buttons with a back button; the project control stays. */
export const ArchiveView: Story = {
  args: { viewMode: 'archive' },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole('button', { name: /ask ai/i })).toBeNull();
    await userEvent.click(canvas.getByRole('button', { name: 'Back to board' }));
    await expect(args.onBack).toHaveBeenCalledTimes(1);
  },
};

/** The epics screen shows the project name as text instead of the project control, and its back button. */
export const EpicsView: Story = {
  args: { viewMode: 'epics' },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole('button', { name: 'alpha' })).toBeNull();
    await expect(canvas.getByText('alpha')).toBeInTheDocument();
    await expect(canvas.getByRole('button', { name: 'Back to board' })).toBeEnabled();
  },
};

/** Without a project list the project control and the slash are hidden, and only the app name is shown. */
export const NoProjectList: Story = {
  args: { projects: null, activeProjectId: null, canAdd: false },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('heading', { name: 'Agent Project' })).toBeInTheDocument();
    await expect(canvas.queryByRole('button', { name: 'alpha' })).toBeNull();
  },
};

/** The passcode screen's header: the app name only, with no menu and no settings button. */
export const WithoutSettings: Story = {
  args: { projects: null, activeProjectId: null, canAdd: false, withSettings: false, menu: false },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('heading', { name: 'Agent Project' })).toBeInTheDocument();
    await expect(canvas.queryByRole('button', { name: /^menu$/i })).toBeNull();
  },
};

/** The board header in Japanese: the labels come from the product's own `ja` resources. */
export const JapaneseBoard: Story = {
  globals: { locale: 'ja' },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('button', { name: /AI に依頼/ })).toBeEnabled();
    const menu = await openMenu(canvasElement);
    await expect(menu.getByRole('menuitem', { name: /ドキュメント/ })).toBeEnabled();
    await expect(menu.getByRole('menuitem', { name: '設定' })).toBeEnabled();
  },
};

/** Each header action calls its own callback once; the menu closes after each choice. */
export const ActionCallbacks: Story = {
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole('button', { name: /ask ai/i }));
    await expect(args.onAdd).toHaveBeenCalledTimes(1);
    await userEvent.click((await openMenu(canvasElement)).getByRole('menuitem', { name: /docs/i }));
    await expect(args.onDocs).toHaveBeenCalledTimes(1);
    await userEvent.click(
      (await openMenu(canvasElement)).getByRole('menuitem', { name: /epics/i }),
    );
    await expect(args.onEpics).toHaveBeenCalledTimes(1);
    await userEvent.click(
      (await openMenu(canvasElement)).getByRole('menuitem', { name: /archive/i }),
    );
    await expect(args.onArchive).toHaveBeenCalledTimes(1);
  },
};

/** The settings item opens the settings slot, and Escape gives the focus back to the menu button. */
export const SettingsOpensAndRestoresFocus: Story = {
  play: async ({ canvasElement }) => {
    const menuButton = within(canvasElement).getByRole('button', { name: /^menu$/i });
    await userEvent.click(
      (await openMenu(canvasElement)).getByRole('menuitem', { name: 'Settings' }),
    );
    await expect(await within(document.body).findByRole('dialog')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(within(document.body).queryByRole('dialog')).toBeNull());
    await waitFor(() => expect(menuButton).toHaveFocus());
  },
};
