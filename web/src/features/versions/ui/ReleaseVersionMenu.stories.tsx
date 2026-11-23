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
    onSelect: fn(async () => undefined),
    onAdd: fn(async () => undefined),
  },
} satisfies Meta<typeof ReleaseVersionMenu>;

export default meta;
type Story = StoryObj<typeof meta>;

const page = () => within(document.body);

export const ReleaseSet: Story = {
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', { name: 'Release version: v1.8.0' }),
    ).toBeVisible();
  },
};

export const NoneSet: Story = {
  args: { release: null },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', { name: 'Release version: Not set' }),
    ).toBeVisible();
  },
};

/** Open: the typed-version row on top, then the list with the release version marked. */
export const Open: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: /Release version/ }));
    const view = page();
    await expect(await view.findByLabelText('New version')).toBeVisible();
    await expect(
      view.getByRole('button', { name: 'Add this version to the project' }),
    ).toBeDisabled();
    await expect(view.getByRole('radio', { name: /v1\.8\.0/ })).toBeChecked();
    await expect(view.getAllByText('Release version').length).toBeGreaterThan(1);
  },
};

export const SelectingAVersion: Story = {
  play: async ({ args, canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: /Release version/ }));
    await userEvent.click(await page().findByRole('radio', { name: /v1\.9\.0/ }));
    await waitFor(() => expect(args.onSelect).toHaveBeenCalledWith('v1.9.0'));
  },
};

export const TypingANewVersion: Story = {
  play: async ({ args, canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: /Release version/ }));
    await userEvent.type(await page().findByLabelText('New version'), 'v2.1.0');
    await userEvent.click(page().getByRole('button', { name: 'Add this version to the project' }));
    await waitFor(() => expect(args.onAdd).toHaveBeenCalledWith('v2.1.0'));
    // Listing a version does not make it the release version.
    await expect(args.onSelect).not.toHaveBeenCalled();
  },
};
