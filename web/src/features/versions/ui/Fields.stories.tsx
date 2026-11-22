import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, waitFor, within } from 'storybook/test';
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

const members = [
  { id: 1, title: 'Plan one', taskStatus: 'todo', plannedVersion: null },
  { id: 2, title: 'Plan two', taskStatus: 'doing', plannedVersion: 'v1.8.0' },
  { id: 3, title: 'Plan three', taskStatus: 'done', plannedVersion: null },
  { id: 4, title: 'Plan four', taskStatus: 'todo', plannedVersion: 'v1.9.0' },
];

const dry: EpicVersionResult = {
  changed: [1],
  unchanged: [2],
  skipped: [3, 4],
  skippedReasons: { '3': 'done', '4': 'planned' },
  dryRun: true,
};

const epicArgs = { members, current: 'v1.8.0', versions: ['v1.8.0', 'v1.9.0'] };

export const EpicDryRun: Story = {
  render: () => (
    <EpicPlanField
      {...epicArgs}
      preview={fn(async () => dry)}
      apply={fn(async () => ({ ...dry, dryRun: false }))}
      onPending={fn()}
    />
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await userEvent.click(view.getByRole('radio', { name: /Make it current/ }));
    await expect(await view.findByText('1 of 4 tasks will change to v1.8.0')).toBeVisible();
    await expect(view.getByText('Skipped: done')).toBeVisible();
    await expect(view.getByText('Skipped: already planned')).toBeVisible();
    await expect(view.getAllByText('No change').length).toBeGreaterThan(1);
  },
};

export const EpicNothingToChange: Story = {
  render: () => (
    <EpicPlanField
      {...epicArgs}
      preview={fn(async () => ({ ...dry, changed: [], unchanged: [1, 2] }))}
      apply={fn(async () => dry)}
      onPending={fn()}
    />
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await userEvent.click(view.getByRole('radio', { name: /Make it current/ }));
    await expect(await view.findByText('No task will change')).toBeVisible();
  },
};

export const EpicCheckFailed: Story = {
  render: () => (
    <EpicPlanField
      {...epicArgs}
      preview={fn(async () => {
        throw new Error('nope');
      })}
      apply={fn(async () => dry)}
      onPending={fn()}
    />
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await userEvent.click(view.getByRole('radio', { name: /Make it current/ }));
    await waitFor(() => expect(view.getByRole('alert')).toBeVisible());
  },
};
