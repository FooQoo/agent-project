import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import { modalDocsParameters } from '../../../.storybook/support/modal-docs';
import { ReleasePlanScreen } from './ReleasePlanScreen';

// A visual mock of the planned "release plan" feature (RELEASE-MANAGEMENT). Static data, no backend.
const meta = {
  title: 'Mocks/ReleasePlan/Desktop',
  component: ReleasePlanScreen,
  parameters: { layout: 'fullscreen', ...modalDocsParameters },
} satisfies Meta<typeof ReleasePlanScreen>;
export default meta;
type Story = StoryObj<typeof meta>;

const page = () => within(document.body);

/** 1. Every card shows its planned version; the current one is marked. Nothing is filtered yet. */
export const BoardAll: Story = {
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', {
        name: /予定版で絞り込み|Filter by planned version/,
      }),
    ).toBeVisible();
  },
};

/** 2. Filtered by "Current (v1.8.0)": the lanes keep only the tasks planned for it. */
export const FilterCurrent: Story = {
  args: { initialFilter: { kind: 'current' } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText(/5 .*13/)).toBeVisible();
  },
};

/** 2. Filtered by one specific version. */
export const FilterSpecificVersion: Story = {
  args: { initialFilter: { kind: 'version', version: 'v1.9.0' } },
};

/** 2. Filtered by "Unplanned": tasks with no planned version. */
export const FilterUnplanned: Story = {
  args: { initialFilter: { kind: 'none' } },
};

/** 2. A version nobody plans for: zero results, with a way back. */
export const FilterZeroResults: Story = {
  args: { initialFilter: { kind: 'version', version: 'v2.0.0' } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getAllByRole('status').length).toBeGreaterThan(1);
    await expect(within(canvasElement).getAllByText(/v2\.0\.0/)[0]).toBeVisible();
  },
};

/** 2. "Current" is chosen but the project has no current version: the board is not filtered, and says why. */
export const NoCurrentVersion: Story = {
  args: { initialCurrent: null, initialFilter: { kind: 'current' } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('alert')).toBeVisible();
  },
};

/** 2. The filter menu, opened. Try the choices; "Current" is disabled-looking text when none is set. */
export const FilterMenuOpen: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole('button', {
        name: /予定版で絞り込み|Filter by planned version/,
      }),
    );
    await expect(await page().findByRole('menu')).toBeVisible();
  },
};

/** 2. The filter menu with no current version set. */
export const FilterMenuNoCurrent: Story = {
  args: { initialCurrent: null },
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole('button', {
        name: /予定版で絞り込み|Filter by planned version/,
      }),
    );
    await expect(await page().findByRole('menu')).toBeVisible();
  },
};

/** 3. A task's detail: set its planned version, or put it back to unplanned. Only this task changes. */
export const DetailVersionPicker: Story = {
  args: { initial: { kind: 'detail', taskId: 13 } },
  play: async () => {
    await expect(await page().findByRole('dialog')).toBeInTheDocument();
  },
};

/** 3. The same picker when the project has no current version: "make it current" is unavailable. */
export const DetailNoCurrent: Story = {
  args: { initialCurrent: null, initial: { kind: 'detail', taskId: 11 } },
};

/** 4. The project setting: which version is current. It is separate from any task's plan. */
export const ProjectCurrentVersion: Story = {
  args: { initial: { kind: 'settings' } },
  play: async () => {
    await expect(await page().findByRole('dialog')).toBeInTheDocument();
  },
};

/** 5. Epic bulk change: the tasks that will change and how many, before anything is saved. */
export const EpicBulkConfirm: Story = {
  args: { initial: { kind: 'bulk', epicId: 'LOGIN-1' } },
  play: async () => {
    await expect(await page().findByRole('dialog')).toBeInTheDocument();
  },
};

/** 5. Epic bulk change with "every task" in scope: tasks already planned elsewhere change too. */
export const EpicBulkAllTasks: Story = {
  args: { initial: { kind: 'bulk', epicId: 'LOGIN-1', scope: 'all', target: 'v1.8.0' } },
};

/** Try it: open a card, pick a version, save; then use the filter. */
export const TryIt: Story = {
  args: { initialFilter: { kind: 'current' } },
  play: async ({ canvasElement }) => {
    await waitFor(() =>
      expect(within(canvasElement).getAllByRole('button').length).toBeGreaterThan(0),
    );
  },
};

/** Japanese, dark. */
export const BoardJaDark: Story = {
  args: { initialFilter: { kind: 'current' } },
  globals: { locale: 'ja', mode: 'dark' },
};
