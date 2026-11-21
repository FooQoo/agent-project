import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, within } from 'storybook/test';
import { modalDocsParameters } from '../../../.storybook/support/modal-docs';
import { ReleasePlanScreen } from './ReleasePlanScreen';

// The remote-access (phone) side of the release plan mock: the same bar and lanes, and every
// dialog as a bottom sheet.
const meta = {
  title: 'Mocks/ReleasePlan/Phone',
  component: ReleasePlanScreen,
  args: { variant: 'phone' },
  parameters: { storyFrame: 'monitor', layout: 'fullscreen', ...modalDocsParameters },
  globals: { width: 'phone-375' },
} satisfies Meta<typeof ReleasePlanScreen>;
export default meta;
type Story = StoryObj<typeof meta>;

const page = () => within(document.body);

/** 1. The bar above the lanes: the filter, and the project's current version kept apart from it. */
export const BoardAll: Story = {
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', {
        name: /予定版で絞り込み|Filter by planned version/,
      }),
    ).toBeVisible();
  },
};

/** 2. Filtered by "Current (v1.8.0)". */
export const FilterCurrent: Story = {
  args: { initialFilter: { kind: 'current' } },
};

/** 2. The filter sheet, opened from the bar. Choosing a row applies it and closes the sheet. */
export const FilterSheet: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole('button', {
        name: /予定版で絞り込み|Filter by planned version/,
      }),
    );
    await expect(await page().findByRole('dialog')).toBeVisible();
  },
};

/** 2. The filter sheet with no current version set. */
export const FilterSheetNoCurrent: Story = {
  args: { initialCurrent: null },
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole('button', {
        name: /予定版で絞り込み|Filter by planned version/,
      }),
    );
    await expect(await page().findByRole('dialog')).toBeVisible();
  },
};

/** 2. A version nobody plans for: zero results. */
export const FilterZeroResults: Story = {
  args: { initialFilter: { kind: 'version', version: 'v2.0.0' } },
};

/** 2. "Current" chosen but not set. */
export const NoCurrentVersion: Story = {
  args: { initialCurrent: null, initialFilter: { kind: 'current' } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('alert')).toBeVisible();
  },
};

/** 3. A task's detail sheet with the version picker. */
export const DetailVersionPicker: Story = {
  args: { initial: { kind: 'detail', taskId: 13 } },
  play: async () => {
    await expect(await page().findByRole('dialog')).toBeVisible();
  },
};

/** 4. The project's current version setting sheet. */
export const ProjectCurrentVersion: Story = {
  args: { initial: { kind: 'settings' } },
  play: async () => {
    await expect(await page().findByRole('dialog')).toBeVisible();
  },
};

/** 5. Epic bulk change as a sheet: targets, count and each change before saving. */
export const EpicBulkConfirm: Story = {
  args: { initial: { kind: 'bulk', epicId: 'LOGIN-1' } },
  play: async () => {
    await expect(await page().findByRole('dialog')).toBeVisible();
  },
};

/** Japanese, dark. */
export const BoardJaDark: Story = {
  args: { initialFilter: { kind: 'current' } },
  globals: { locale: 'ja', mode: 'dark', width: 'phone-375' },
};
