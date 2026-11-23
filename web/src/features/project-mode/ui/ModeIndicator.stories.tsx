import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
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

/** Asleep with held questions: the chip counts them. */
export const AsleepWithHeldQuestions: Story = {
  args: { mode: { ...SPARRING, mode: 'sleep', since: '2026-11-24T01:00:00Z', heldCount: 2 } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText('2 questions held')).toBeVisible();
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

const brainstorming = {
  ...SPARRING,
  mode: 'brainstorm',
  manual: 'brainstorm',
  target: { type: 'epic', id: 'INTERACTION-MODES' },
} as const;

/** Brainstorming: the chip names the target and the End button ends it. */
export const Brainstorm: Story = {
  args: { mode: brainstorming, onEndBrainstorm: fn() },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('status')).toHaveTextContent('Brainstorm: INTERACTION-MODES');
    await userEvent.click(canvas.getByRole('button', { name: 'End brainstorm' }));
    await expect(args.onEndBrainstorm).toHaveBeenCalled();
  },
};

/** A narrow header: the long target is cut with an ellipsis and the End button stays. */
export const BrainstormNarrow: Story = {
  args: { mode: brainstorming, onEndBrainstorm: fn() },
  decorators: [
    (Story) => (
      <div style={{ width: 180, display: 'flex' }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', { name: 'End brainstorm' }),
    ).toBeVisible();
  },
};
