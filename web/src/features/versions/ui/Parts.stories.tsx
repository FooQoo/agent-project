import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, within } from 'storybook/test';
import { PlanContext, PlanTag } from './PlanContext';
import { VersionTag } from './VersionTag';

const meta = {
  title: 'Versions/Parts',
  parameters: { layout: 'centered' },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

/** The three looks of a card's planned-version tag. */
export const VersionTagStates: Story = {
  render: () => (
    <div style={{ display: 'flex', gap: 8 }}>
      <VersionTag planned="v1.9.0" release="v1.8.0" />
      <VersionTag planned="v1.8.0" release="v1.8.0" />
      <VersionTag planned={null} release="v1.8.0" />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(view.getByText('v1.9.0')).toBeVisible();
    await expect(view.getByText('Release version')).toBeVisible();
    await expect(view.getByText('Unplanned')).toBeVisible();
  },
};

/** The phone's read-only tag appears only where a screen provides the project's versions. */
export const PlanTagInContext: Story = {
  render: () => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      <PlanContext.Provider value={{ versions: ['v1.8.0'], release: 'v1.8.0' }}>
        <div data-testid="with">
          <PlanTag planned="v1.8.0" />
        </div>
      </PlanContext.Provider>
      <div data-testid="without">
        <PlanTag planned="v1.8.0" />
      </div>
    </div>
  ),
  play: async ({ canvasElement }) => {
    const view = within(canvasElement);
    await expect(within(view.getByTestId('with')).getByText('v1.8.0')).toBeVisible();
    await expect(within(view.getByTestId('without')).queryByText('v1.8.0')).toBeNull();
  },
};
