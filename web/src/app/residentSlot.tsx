import type { ReactNode } from 'react';
import { requestResidentDialog } from '../features/resident-sessions/events';
import { ConnectedResidentEntryRow } from '../features/resident-sessions/ui/ResidentEntryRow';
import { ConnectedResidentIndicator } from '../features/resident-sessions/ui/ResidentFooter';
import { ResidentHost } from '../features/resident-sessions/ui/ResidentHost';
import { isProjectId } from './model/projectSession';

/** The sample-data mode and the demo have no server, so they have no session to show or start. */
const serverless = import.meta.env.MODE === 'mock' || import.meta.env.MODE === 'demo';

/** Whether the resident-session row, dialog and footer indicator exist for this project. */
export const residentAvailable = (projectId: string | null): projectId is string =>
  !serverless && isProjectId(projectId);

/**
 * The footer indicator of the board. The phone layout asks for `displayOnly` because its status
 * line has no room for a button; the phone settings sheet opens the full dialog instead.
 */
export function residentFooterSlot(
  projectId: string | null,
  { displayOnly = false }: { displayOnly?: boolean } = {},
): ReactNode {
  if (!residentAvailable(projectId)) return undefined;
  return (
    <ConnectedResidentIndicator
      projectId={projectId}
      remote={displayOnly}
      withState={displayOnly}
    />
  );
}

/** The dialog the footer and the settings row open, available on both local and remote views. */
export function ResidentDialogHost({ projectId }: { projectId: string | null }) {
  if (!residentAvailable(projectId)) return null;
  return <ResidentHost key={projectId} projectId={projectId} />;
}

/**
 * The settings row that opens the resident-session dialog, for the PC and the phone settings
 * alike. It closes the settings dialog first, so only one dialog is on the screen.
 */
export function ResidentSettingsRow({
  projectId,
  onClose,
}: {
  projectId: string | null;
  onClose: () => void;
}) {
  if (!residentAvailable(projectId)) return null;
  return (
    <ConnectedResidentEntryRow
      projectId={projectId}
      onOpen={() => {
        onClose();
        requestResidentDialog();
      }}
    />
  );
}
