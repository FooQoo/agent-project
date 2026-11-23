import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
import { SPARRING, type ProjectMode } from '../model';
import { ModeSection } from './ModeSection';

const SCHEDULE = { enabled: true, start: '23:00', end: '08:00' };
const asleep: ProjectMode = { ...SPARRING, mode: 'sleep', since: '2026-11-24T01:00:00Z' };

// The mode switch of the settings dialog, below the language and the theme. The two options have
// the same size, as the language options do.
const meta = {
  title: 'ProjectMode/ModeSection',
  component: ModeSection,
  args: { mode: SPARRING, onChoose: fn(), onSaveSchedule: fn(), onFollowSchedule: fn() },
  decorators: [
    (Story) => (
      <div style={{ maxWidth: 560, padding: 16 }}>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof ModeSection>;
export default meta;
type Story = StoryObj<typeof meta>;

/** The default: the agent listens and asks on the board. */
export const Sparring: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('radio', { name: 'Sparring' })).toBeChecked();
    await expect(canvas.getByText(/asks you on the board/)).toBeVisible();
  },
};

/** Sleep: questions are paused. */
export const Sleep: Story = {
  args: { mode: asleep },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('radio', { name: 'Sleep' })).toBeChecked();
    await expect(canvas.getByText(/Questions to you are paused/)).toBeVisible();
  },
};

/** Choosing the other option asks the parent to save it. */
export const Choose: Story = {
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole('radio', { name: 'Sleep' }));
    await expect(args.onChoose).toHaveBeenCalledWith('sleep');
  },
};

/** The mode is not known yet, so the switch cannot be used. */
export const Unknown: Story = {
  args: { mode: null },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('radio', { name: 'Sleep' })).toBeDisabled();
  },
};

/** A refused save leaves the value as it was and says so. */
export const SaveFailed: Story = {
  args: { failed: true },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('alert')).toBeVisible();
  },
};

/** The window decides: asleep until it ends, with the schedule on. */
export const BySchedule: Story = {
  args: {
    mode: {
      ...SPARRING,
      mode: 'sleep',
      modeSource: 'schedule',
      until: '2026-11-24T08:00:00',
      schedule: SCHEDULE,
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText(/Asleep by the schedule until/)).toBeVisible();
    await expect(canvas.getByLabelText('Sleep on a schedule')).toBeChecked();
    await expect(canvas.getByLabelText('From')).toHaveValue('23:00');
  },
};

/** A manual mode overrides the window: the switch offers to follow the schedule again. */
export const ManualOverSchedule: Story = {
  args: { mode: { ...SPARRING, manual: 'sparring', schedule: SCHEDULE } },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText(/Set by hand/)).toBeVisible();
    await userEvent.click(canvas.getByRole('button', { name: 'Follow the schedule' }));
    await expect(args.onFollowSchedule).toHaveBeenCalled();
  },
};

/** Turning the window on saves it with its times. */
export const TurnOnSchedule: Story = {
  args: { mode: { ...SPARRING, schedule: { ...SCHEDULE, enabled: false } } },
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByLabelText('Sleep on a schedule'));
    await expect(args.onSaveSchedule).toHaveBeenCalledWith(SCHEDULE);
  },
};
