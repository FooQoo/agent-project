import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fireEvent, fn, userEvent, within } from 'storybook/test';
import { SPARRING, type ProjectMode } from '../model';
import { ModeSection } from './ModeSection';

const SCHEDULE = { enabled: true, start: '23:00', end: '08:00' };

// Sleep settings in the settings dialog.
const meta = {
  title: 'ProjectMode/ModeSection',
  component: ModeSection,
  args: { mode: SPARRING, onSaveSchedule: fn(), onFollowSchedule: fn() },
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

/** The project settings have yet to load. */
export const Unknown: Story = {
  args: { mode: null },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('checkbox', { name: 'Sleep on a schedule' })).toBeDisabled();
    await expect(canvas.getByLabelText('From')).toBeDisabled();
    await expect(canvas.getByLabelText('Until')).toBeDisabled();
  },
};

/** A refused save leaves the value as it was and says so. */
export const SaveFailed: Story = {
  args: { failed: true },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('alert')).toBeVisible();
  },
};

/** The saved daily window is enabled and both times can be edited. */
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
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByLabelText('Sleep on a schedule')).toBeChecked();
    await expect(canvas.getByLabelText('From')).toHaveValue('23:00');
    await expect(canvas.getByLabelText('Until')).toHaveValue('08:00');
    fireEvent.change(canvas.getByLabelText('From'), { target: { value: '22:00' } });
    await expect(args.onSaveSchedule).toHaveBeenCalledWith({ ...SCHEDULE, start: '22:00' });
    fireEvent.change(canvas.getByLabelText('Until'), { target: { value: '07:30' } });
    await expect(args.onSaveSchedule).toHaveBeenCalledWith({ ...SCHEDULE, end: '07:30' });
  },
};

/** A manual mode overrides the window: the settings offer to follow the schedule again. */
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

const brainstorming: ProjectMode = {
  ...SPARRING,
  mode: 'brainstorm',
  manual: 'brainstorm',
  target: { type: 'task', id: '680' },
};

/** An active brainstorm can still be ended from settings. */
export const Brainstorm: Story = {
  args: { mode: brainstorming, onEndBrainstorm: fn() },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText('Brainstorming #680')).toBeVisible();
    await userEvent.click(canvas.getByRole('button', { name: 'End brainstorm' }));
    await expect(args.onEndBrainstorm).toHaveBeenCalled();
  },
};
