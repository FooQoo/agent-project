import { useState } from 'react';
import { parseAsString, useQueryState } from 'nuqs';
import { ConnectionEntryRow } from '../features/monitor-connections/ui/ConnectionEntryRow';
import { ConnectionSettings } from '../features/monitor-connections/ui/ConnectionSettings';
import { AgentConnectionEntryRow } from '../features/agent-connection/AgentConnectionEntryRow';
import { AgentConnectionSettings } from '../features/agent-connection/AgentConnectionSettings';
import { SettingsDialog } from '../features/settings/ui/SettingsDialog';
import { csrfHeaders, readToken } from '../shared/api/http';
import { TutorialEntryRow } from './onboarding/TutorialEntryRow';
import { requestTutorial } from './onboarding/tutorialRequest';
import { ModeSettingsSection } from './modeSlot';
import { NotificationSettingsRow } from './notificationSlot';
import { ResidentSettingsRow } from './residentSlot';
import { usesRemoteSession } from './remote/session';
import type { PreferencesEndpoint } from '../shared/preferences';
import type { SettingsSlotState } from '../shared/ui/app-header/Header';

/** The board saves the language and theme on its own server; the sample-data mode and the demo have none. */
const boardPreferences: PreferencesEndpoint | undefined =
  import.meta.env.MODE === 'mock' || import.meta.env.MODE === 'demo'
    ? undefined
    : { url: '/api/preferences', token: readToken, headers: csrfHeaders };
/**
 * A phone connects to an `agpj serve` on this computer; the demo on a static site has none. A
 * page that itself came through the ngrok entrance does not show the connection either: it is
 * started and stopped on the computer, and its address and passcode stay off the remote screen.
 */
const phoneConnection = () => import.meta.env.MODE !== 'demo' && !usesRemoteSession();

/**
 * The settings dialog of the PC board, the remote connection dialog, and the resident-session and notification dialogs its entry rows open.
 * The dialogs are siblings: the settings dialog closes before the connection dialog opens,
 * so only one of them is ever on the screen.
 */
export function BoardSettings({ open, onOpenChange, restoreFocusRef }: SettingsSlotState) {
  const [connectionOpen, setConnectionOpen] = useState(false);
  const [agentConnectionOpen, setAgentConnectionOpen] = useState(false);
  // The project the board shows, from the same `project` query the board reads.
  const [projectId] = useQueryState('project', parseAsString);
  const withConnection = phoneConnection();
  const withAgentConnection = withConnection && import.meta.env.MODE !== 'mock';
  return (
    <>
      <SettingsDialog
        open={open}
        onOpenChange={onOpenChange}
        restoreFocusRef={restoreFocusRef}
        preferencesEndpoint={boardPreferences}
        extra={
          <>
            <ModeSettingsSection projectId={projectId} />
            <div className="settings__entries">
              <TutorialEntryRow
                onStart={() => {
                  onOpenChange(false);
                  requestTutorial();
                }}
              />
              {withConnection ? (
                <ConnectionEntryRow
                  onOpen={() => {
                    onOpenChange(false);
                    setConnectionOpen(true);
                  }}
                />
              ) : null}
              {withAgentConnection ? (
                <AgentConnectionEntryRow
                  onOpen={() => {
                    onOpenChange(false);
                    setAgentConnectionOpen(true);
                  }}
                />
              ) : null}
              <ResidentSettingsRow projectId={projectId} onClose={() => onOpenChange(false)} />
              {boardPreferences ? (
                <NotificationSettingsRow onClose={() => onOpenChange(false)} />
              ) : null}
            </div>
          </>
        }
      />
      {withConnection ? (
        <ConnectionSettings
          open={connectionOpen}
          onOpenChange={setConnectionOpen}
          restoreFocusRef={restoreFocusRef}
          projectId={projectId}
        />
      ) : null}
      {withAgentConnection && agentConnectionOpen ? (
        <AgentConnectionSettings
          key={projectId}
          onOpenChange={setAgentConnectionOpen}
          projectId={projectId}
          restoreFocusRef={restoreFocusRef}
        />
      ) : null}
    </>
  );
}
