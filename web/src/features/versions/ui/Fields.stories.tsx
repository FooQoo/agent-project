import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
import type { PendingSave } from '../../../shared/ui/editor/planField';
import { type EpicVersionResult, PlannedVersionChangedError } from '../api/versions';
import { EpicPlanField } from './EpicPlanField';
import { PlannedVersionPicker } from './PlannedVersionPicker';

const meta = {
  title: 'Versions/Fields',
  parameters: { layout: 'padded' },
  // The fields live in the narrow right column of the editors.
  decorators: [
    (Story) => (
      <div style={{ width: 300 }}>
        <Story />
      </div>
    ),
  ],
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

/** The change the picker last reported through `onPending`. */
let pending: PendingSave | null = null;
const refetch = fn();

const pickerArgs = {
  taskId: 7,
  planned: null,
  current: 'v1.8.0',
  versions: ['v1.8.0', 'v1.9.0'],
};

export const PickerCurrentEnabled: Story = {
  render: () => (
    <PlannedVersionPicker {...pickerArgs} onSave={fn(async () => undefined)} onPending={fn()} />
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByRole('radio', { name: /Make it current/ })).toBeEnabled();
    await expect(view.getByRole('radio', { name: /Leave unplanned/ })).toBeChecked();
  },
};

export const PickerCurrentDisabled: Story = {
  render: () => (
    <PlannedVersionPicker
      {...pickerArgs}
      current={null}
      onSave={fn(async () => undefined)}
      onPending={fn()}
    />
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByRole('radio', { name: /Make it current/ })).toBeDisabled();
    await expect(view.getByText('Unavailable: no current version is set')).toBeVisible();
  },
};

/** A change made elsewhere refuses the save: the newer value is shown and the board is read again. */
export const PickerConflict: Story = {
  render: () => (
    <PlannedVersionPicker
      {...pickerArgs}
      onSave={async () => {
        throw new PlannedVersionChangedError('changed', 'v1.9.0');
      }}
      onRefetch={refetch}
      onPending={(next) => {
        pending = next;
      }}
    />
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    refetch.mockClear();
    await userEvent.click(view.getByRole('radio', { name: /Make it current/ }));
    await expect(pending).not.toBeNull();
    await expect((pending as PendingSave | null)?.()).rejects.toBeInstanceOf(
      PlannedVersionChangedError,
    );
    await expect(await view.findByRole('alert')).toHaveTextContent('v1.9.0');
    await expect(refetch).toHaveBeenCalledTimes(1);
  },
};

const result: EpicVersionResult = {
  changed: [1, 2],
  unchanged: [],
  skipped: [3],
  skippedReasons: { '3': 'done' },
  dryRun: false,
};

const epicArgs = { current: 'v1.8.0' as string | null, versions: ['v1.8.0', 'v1.9.0'] };

export const EpicDefaultNoChange: Story = {
  render: () => <EpicPlanField {...epicArgs} apply={fn(async () => result)} onPending={fn()} />,
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByRole('radio', { name: 'No change' })).toBeChecked();
    await expect(view.getByRole('radio', { name: /Make it current/ })).toBeVisible();
    await expect(view.getByRole('radio', { name: 'v1.9.0' })).toBeVisible();
  },
};

export const EpicPickingAVersion: Story = {
  render: () => <EpicPlanField {...epicArgs} apply={fn(async () => result)} onPending={fn()} />,
  play: async ({ canvasElement, args }) => {
    const view = within(canvasElement);
    await userEvent.click(view.getByRole('radio', { name: 'v1.9.0' }));
    await expect(view.getByRole('radio', { name: 'v1.9.0' })).toBeChecked();
    void args;
  },
};

export const EpicCurrentUnavailable: Story = {
  render: () => (
    <EpicPlanField {...epicArgs} current={null} apply={fn(async () => result)} onPending={fn()} />
  ),
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('radio', { name: /Make it current/ }),
    ).toBeDisabled();
  },
};
