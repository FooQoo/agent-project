import { CheckIcon, ChevronDownIcon } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AssignedAgent } from '../../../shared/api/contracts';
import { onDismissSettingsSheets } from '../../../shared/lib/sheets/dismissSheets';
import { useOptionalDataCore } from '../../../shared/data/react/provider';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '../../../shared/ui/dropdown-menu';
import { Surface } from '../../../shared/ui/surface/Surface';
import { TransientNotice } from '../../../shared/ui/transient-notice/TransientNotice';
import { AgentAvatar, AgentIdentity } from '../../../shared/ui/agent/AgentIdentity';
import { loadAgents, type Agent } from '../../../shared/api/agents';
import { assignAgent } from '../api/edit';
import './task-agent.css';

export function TaskAgentPicker({
  agent,
  projectId,
  taskId,
  phone = false,
  readOnly = false,
}: {
  agent: AssignedAgent;
  projectId: string;
  taskId: number;
  phone?: boolean;
  readOnly?: boolean;
}) {
  const { t } = useTranslation();
  const core = useOptionalDataCore();
  const [open, setOpen] = useState(false);
  const [options, setOptions] = useState<Agent[] | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<{ message: string; tone: 'success' | 'error' } | null>(null);
  const dismissNotice = useCallback(() => setNotice(null), []);

  useEffect(() => (phone ? onDismissSettingsSheets(() => setOpen(false)) : undefined), [phone]);

  useEffect(() => {
    if (!open) return;
    let active = true;
    void loadAgents(projectId).then(
      (agents) => {
        if (active) setOptions(agents);
      },
      () => {
        if (active) setLoadFailed(true);
      },
    );
    return () => {
      active = false;
    };
  }, [open, projectId]);

  const openPicker = (next: boolean) => {
    if (next) {
      setOptions(null);
      setLoadFailed(false);
    }
    setOpen(next);
  };

  const choose = async (id: string) => {
    const chosen = options?.find((option) => option.id === id);
    openPicker(false);
    if (!chosen || chosen.id === agent.id || saving) return;
    setSaving(true);
    try {
      await assignAgent(projectId, taskId, chosen.id);
      await core?.refreshSummary(projectId);
      setNotice({ message: t('agentConnection.assigned', { name: chosen.name }), tone: 'success' });
    } catch {
      setNotice({ message: t('agentConnection.assignFailed'), tone: 'error' });
    } finally {
      setSaving(false);
    }
  };

  const content = (
    <>
      <AgentAvatar {...agent} />
      <span>{agent.name}</span>
    </>
  );
  if (readOnly || !core) return <span className="task-agent">{content}</span>;
  const trigger = (
    <button
      type="button"
      className="task-agent task-agent--trigger"
      aria-label={`${t('agentConnection.reassign')} · ${agent.name}`}
      disabled={saving}
      onClick={phone ? () => openPicker(true) : undefined}
    >
      {content}
      <ChevronDownIcon size={14} aria-hidden="true" />
    </button>
  );
  const loading = options === null && (
    <p className="task-agent__loading" role={loadFailed ? 'alert' : 'status'}>
      {t(loadFailed ? 'agentConnection.loadFailed' : 'agentConnection.loading')}
    </p>
  );

  return (
    <>
      {phone ? (
        <>
          {trigger}
          {open && (
            <Surface
              kind="sheet"
              title={t('agentConnection.reassign')}
              closeLabel={t('dialog.close')}
              onClose={() => openPicker(false)}
            >
              {loading}
              <ul className="task-agent__options">
                {options?.map((option) => (
                  <li key={option.id}>
                    <button
                      type="button"
                      className="task-agent__option"
                      aria-current={option.id === agent.id}
                      onClick={() => void choose(option.id)}
                    >
                      <AgentIdentity agent={option} />
                      {option.id === agent.id && <CheckIcon size={16} aria-hidden="true" />}
                    </button>
                  </li>
                ))}
              </ul>
            </Surface>
          )}
        </>
      ) : (
        <DropdownMenu open={open} onOpenChange={openPicker}>
          <DropdownMenuTrigger asChild>{trigger}</DropdownMenuTrigger>
          <DropdownMenuContent className="task-agent__menu">
            {loading}
            <DropdownMenuRadioGroup value={agent.id} onValueChange={(id) => void choose(id)}>
              {options?.map((option) => (
                <DropdownMenuRadioItem key={option.id} value={option.id}>
                  <AgentIdentity agent={option} />
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
      <TransientNotice
        message={notice?.message ?? null}
        tone={notice?.tone}
        onDismiss={dismissNotice}
      />
    </>
  );
}
