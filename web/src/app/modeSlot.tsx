import type { ReactNode } from 'react';
import { BrainstormAction } from '../features/project-mode/ui/BrainstormAction';
import type { BrainstormTarget } from '../features/project-mode/model';
import { ModeIndicator } from '../features/project-mode/ui/ModeIndicator';
import { ConnectedModeSection } from '../features/project-mode/ui/ModeSection';
import { isProjectId } from './model/projectSession';

/** The sample-data mode and the demo have no server, so they have no mode to read or save. */
const serverless = import.meta.env.MODE === 'mock' || import.meta.env.MODE === 'demo';

/** Whether the sleep settings and the header indicator exist for this project. */
export const modeAvailable = (projectId: string | null): projectId is string =>
  !serverless && isProjectId(projectId);

/** The sleep chip of the header; absent where there is no server to ask. */
export function modeIndicatorSlot(projectId: string | null): ReactNode {
  return modeAvailable(projectId) ? <ModeIndicator projectId={projectId} /> : undefined;
}

/** The sleep settings, for the PC and the phone alike. Reachable through the remote entrance too. */
export function ModeSettingsSection({ projectId }: { projectId: string | null }) {
  return modeAvailable(projectId) ? <ConnectedModeSection projectId={projectId} /> : null;
}

/** The Brainstorm action of a card or an epic; absent where there is no server to ask. */
export function brainstormActionSlot(
  projectId: string | null,
  target: BrainstormTarget,
): ReactNode {
  return modeAvailable(projectId) ? (
    <BrainstormAction projectId={projectId} target={target} />
  ) : undefined;
}
