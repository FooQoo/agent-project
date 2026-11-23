import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, fn, userEvent, within } from 'storybook/test';
import { SPARRING } from '../model';
import { BrainstormActionView } from './BrainstormAction';

const target = { type: 'task', id: '680' } as const;

// The Brainstorm action in the toolbar of a card or an epic.
const meta = {
  title: 'ProjectMode/BrainstormAction',
  component: BrainstormActionView,
  args: { mode: SPARRING, target, onStart: fn(), onEnd: fn() },
} satisfies Meta<typeof BrainstormActionView>;
export default meta;
type Story = StoryObj<typeof meta>;

/** Sparring: the action starts a brainstorm on this target. */
export const Idle: Story = {
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole('button', { name: 'Brainstorm #680' }));
    await expect(args.onStart).toHaveBeenCalled();
  },
};

/** This target is being brainstormed: the action says so and offers to end it. */
export const Active: Story = {
  args: { mode: { ...SPARRING, mode: 'brainstorm', manual: 'brainstorm', target } },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText('Brainstorming')).toBeVisible();
    await userEvent.click(canvas.getByRole('button', { name: 'End' }));
    await expect(args.onEnd).toHaveBeenCalled();
  },
};

/** Another target is being brainstormed: starting here moves the brainstorm. */
export const OtherTarget: Story = {
  args: {
    mode: {
      ...SPARRING,
      mode: 'brainstorm',
      manual: 'brainstorm',
      target: { type: 'epic', id: 'X' },
    },
  },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', { name: 'Brainstorm #680' }),
    ).toBeEnabled();
  },
};

/** Asleep, by the schedule or by hand: sleep wins, so the action is disabled and says why. */
export const Asleep: Story = {
  args: { mode: { ...SPARRING, mode: 'sleep', since: '2026-11-24T01:00:00Z' } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('button', { name: 'Brainstorm #680' })).toBeDisabled();
    await expect(
      canvas.getByText('Brainstorming is unavailable in sleep mode.'),
    ).toBeInTheDocument();
  },
};

/** The mode is not known yet: the action cannot be used. */
export const Unknown: Story = {
  args: { mode: null },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole('button', { name: 'Brainstorm #680' }),
    ).toBeDisabled();
  },
};

/** The start was refused (the card or epic is gone): the action says so. */
export const Failed: Story = {
  args: { failed: true },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole('alert')).toBeVisible();
  },
};
