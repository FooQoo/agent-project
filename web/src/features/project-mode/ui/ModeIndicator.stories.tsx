import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { SPARRING } from '../model';
import { ModeIndicatorView } from './ModeIndicator';

// The chip the header shows while a project is asleep.
const meta = {
  title: 'ProjectMode/ModeIndicator',
  component: ModeIndicatorView,
  args: { mode: { ...SPARRING, mode: 'sleep', since: '2026-11-24T01:00:00Z' } },
} satisfies Meta<typeof ModeIndicatorView>;
export default meta;
type Story = StoryObj<typeof meta>;

/** Asleep: the chip is shown. */
export const Asleep: Story = {
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('status')).toBeVisible();
  },
};

/** Sparring is the default and shows nothing. */
export const Sparring: Story = {
  args: { mode: SPARRING },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole('status')).toBeNull();
  },
};

/** Asleep by the schedule: the chip also says when the window ends. */
export const AsleepBySchedule: Story = {
  args: {
    mode: { ...SPARRING, mode: 'sleep', modeSource: 'schedule', until: '2026-11-24T08:00:00' },
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText(/^until /)).toBeInTheDocument();
  },
};
