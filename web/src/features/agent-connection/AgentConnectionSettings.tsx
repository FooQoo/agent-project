import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../shared/ui/button';
import { Surface } from '../../shared/ui/surface/Surface';
import { AgentCodePanel } from './AgentCodePanel';
import { AgentIdentity } from '../../shared/ui/agent/AgentIdentity';
import { AgentRegistrationForm } from './AgentRegistrationForm';
import { loadAgents, type Agent } from '../../shared/api/agents';
import { prepareDefaultAgent, saveAgent } from './api';
import './agent-connection.css';

export function AgentConnectionSettings({
  onOpenChange,
  projectId,
  restoreFocusRef,
  kind = 'dialog',
  canIssueCode = kind === 'dialog',
}: {
  onOpenChange: (open: boolean) => void;
  projectId: string;
  restoreFocusRef?: { current: HTMLElement | null };
  kind?: 'dialog' | 'sheet';
  canIssueCode?: boolean;
}) {
  const { t } = useTranslation();
  const [agents, setAgents] = useState<Agent[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [edit, setEdit] = useState<Agent | 'new' | null>(null);
  const [connect, setConnect] = useState<Agent | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [prepareFailed, setPrepareFailed] = useState(false);

  useEffect(() => {
    let active = true;
    let request = 0;
    const refresh = () => {
      const current = ++request;
      void loadAgents(projectId).then(
        (loaded) => {
          if (!active || current !== request) return;
          setAgents(loaded);
          setFailed(false);
        },
        () => {
          if (active && current === request) setFailed(true);
        },
      );
    };
    refresh();
    window.addEventListener('focus', refresh);
    return () => {
      active = false;
      window.removeEventListener('focus', refresh);
    };
  }, [projectId]);

  const save = async (value: Pick<Agent, 'name' | 'role'>) => {
    const id = edit && edit !== 'new' ? edit.id : undefined;
    const saved = await saveAgent(projectId, value, id);
    setAgents((current) =>
      id
        ? (current ?? []).map((item) => (item.id === id ? saved : item))
        : [...(current ?? []), saved],
    );
  };

  const prepare = async () => {
    if (preparing) return;
    setPreparing(true);
    setPrepareFailed(false);
    try {
      await prepareDefaultAgent(projectId);
      setAgents(await loadAgents(projectId));
    } catch {
      setPrepareFailed(true);
    } finally {
      setPreparing(false);
    }
  };

  return (
    <Surface
      kind={kind}
      title={t('agentConnection.title')}
      closeLabel={t('dialog.close')}
      onClose={() => onOpenChange(false)}
      returnFocusTo={() => restoreFocusRef?.current ?? null}
    >
      {failed ? <p role="alert">{t('agentConnection.loadFailed')}</p> : null}
      {!agents && !failed ? <p role="status">{t('agentConnection.loading')}</p> : null}
      {agents && (
        <>
          {!agents.some((agent) => agent.isDefault) && (
            <div className="agent-settings__default">
              <p>{t('agentConnection.defaultHelp')}</p>
              <Button
                type="button"
                variant="outline"
                disabled={preparing}
                onClick={() => void prepare()}
              >
                {t('agentConnection.prepareDefault')}
              </Button>
              {prepareFailed && <p role="alert">{t('agentConnection.saveFailed')}</p>}
            </div>
          )}
          {edit === 'new' ? (
            <AgentRegistrationForm onSave={save} onClose={() => setEdit(null)} />
          ) : (
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setConnect(null);
                setEdit('new');
              }}
            >
              {t('agentConnection.add')}
            </Button>
          )}
          <ul className="agent-settings__list">
            {agents.map((agent) => (
              <li key={agent.id}>
                <AgentIdentity agent={agent} />
                {edit && edit !== 'new' && edit.id === agent.id ? (
                  <AgentRegistrationForm
                    agent={agent}
                    onSave={save}
                    onClose={() => setEdit(null)}
                  />
                ) : (
                  <div className="agent-settings__actions">
                    <Button
                      type="button"
                      size="s"
                      variant="outline"
                      onClick={() => {
                        setConnect(null);
                        setEdit(agent);
                      }}
                    >
                      {t('agentConnection.edit')}
                    </Button>
                    {canIssueCode && (
                      <Button
                        type="button"
                        size="s"
                        variant="outline"
                        onClick={() => {
                          setEdit(null);
                          setConnect(agent);
                        }}
                      >
                        {t(
                          agent.connection
                            ? 'agentConnection.switch'
                            : 'agentConnection.prepareCode',
                        )}
                      </Button>
                    )}
                  </div>
                )}
                {connect?.id === agent.id && (
                  <AgentCodePanel
                    key={agent.id}
                    agent={connect}
                    projectId={projectId}
                    onClose={() => setConnect(null)}
                  />
                )}
              </li>
            ))}
          </ul>
          {!canIssueCode && <p>{t('agentConnection.codeOnComputer')}</p>}
        </>
      )}
    </Surface>
  );
}
