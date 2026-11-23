import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, waitFor, within } from 'storybook/test';
import { ReleaseVersionMenu } from './ReleaseVersionMenu';

const meta = {
  title: 'Versions/ReleaseVersionMenu',
  component: ReleaseVersionMenu,
  parameters: { layout: 'padded' },
  args: {
    versions: ['v1.8.0', 'v1.9.0', 'v2.0.0'],
    release: 'v1.8.0',
    filter: 'v1.8.0',
    onFilter: fn(),
    onMakeRelease: fn(async () => undefined),
    onAdd: fn(async () => undefined),
  },
} satisfies Meta<typeof ReleaseVersionMenu>;

export default meta;
type Story = StoryObj<typeof meta>;

const page = () => within(document.body);

export const FilteredByRelease: Story = {
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', { name: 'Version filter: v1.8.0' }),
    ).toBeVisible();
  },
};

export const ShowingAll: Story = {
  args: { release: null, filter: null },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', { name: 'Version filter: All' }),
    ).toBeVisible();
  },
};

/** Open on another version: All on top, the release version starred, the release button at the bottom. */
export const Open: Story = {
  args: { filter: 'v2.0.0' },
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: /Version filter/ }));
    const view = page();
    await expect(await view.findByLabelText('New version')).toBeVisible();
    await expect(view.getByRole('radio', { name: 'All' })).not.toBeChecked();
    await expect(view.getByRole('radio', { name: /v2\.0\.0/ })).toBeChecked();
    await expect(
      view.getByRole('button', { name: 'Make the shown v2.0.0 the release version' }),
    ).toBeVisible();
  },
};

export const ChoosingAVersion: Story = {
  play: async ({ args, canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: /Version filter/ }));
    await userEvent.click(await page().findByRole('radio', { name: /v1\.9\.0/ }));
    await waitFor(() => expect(args.onFilter).toHaveBeenCalledWith('v1.9.0'));
    await expect(args.onMakeRelease).not.toHaveBeenCalled();
  },
};

export const TypingANewVersion: Story = {
  play: async ({ args, canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: /Version filter/ }));
    await userEvent.type(await page().findByLabelText('New version'), 'v2.1.0');
    await userEvent.click(page().getByRole('button', { name: 'Add this version to the project' }));
    await waitFor(() => expect(args.onAdd).toHaveBeenCalledWith('v2.1.0'));
    await expect(args.onFilter).not.toHaveBeenCalled();
  },
};

/** The phone's bottom sheet: it filters and lists versions, marks the release version, and has no release button. */
export const PhoneSheet: Story = {
  args: { kind: 'sheet', filter: 'v2.0.0', onMakeRelease: undefined },
  globals: { viewport: { value: 'mobile1' } },
  play: async ({ args, canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: /Version filter/ }));
    const view = page();
    await expect(await view.findByLabelText('New version')).toBeVisible();
    await expect(view.getByRole('radio', { name: /v1\.8\.0/ }).parentElement).toHaveTextContent(
      '\u2605',
    );
    await expect(view.queryByRole('button', { name: /release version/i })).toBeNull();
    await userEvent.click(view.getByRole('radio', { name: /v1\.9\.0/ }));
    await waitFor(() => expect(args.onFilter).toHaveBeenCalledWith('v1.9.0'));
  },
};
