import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Dialog, DialogContent, DialogTitle } from '../../shared/ui/dialog';
import { Button } from '../../shared/ui/button';
import { WriteError } from '../../shared/api/writeErrors';
import { AgentList } from './AgentList';
import { defaultAgentId, issueConnectCode, type ConnectCode } from './api';
import { useAgents } from './useAgents';
import '../../shared/styles/modal.css';

export function AgentConnectionSettings({
  onOpenChange,
  projectId,
  restoreFocusRef,
}: {
  onOpenChange: (open: boolean) => void;
  projectId: string | null;
  restoreFocusRef?: { current: HTMLElement | null };
}) {
  const { t } = useTranslation();
  const [code, setCode] = useState<ConnectCode | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);
  const { query, register, changeTool } = useAgents(projectId);
  const agents = query.data ?? [];
  const agentId = defaultAgentId(agents);
  const loading = projectId !== null && query.isPending;
  const writing = register.isPending || changeTool.isPending;
  const writeFailed = register.isError || changeTool.isError;

  useEffect(() => {
    if (!code) return;
    const expiry = window.setTimeout(
      () => {
        setCode(null);
        setCopied(false);
      },
      Math.max(0, Date.parse(code.expiresAt) - Date.now()),
    );
    return () => window.clearTimeout(expiry);
  }, [code]);

  const issue = async () => {
    if (!projectId || !agentId || busy) return;
    setBusy(true);
    setError('');
    setCode(null);
    setCopied(false);
    try {
      setCode(await issueConnectCode(projectId, agentId));
    } catch (reason) {
      setError(
        reason instanceof WriteError && reason.code === 'agent_busy'
          ? t('agentConnection.busy')
          : t('agentConnection.issueFailed'),
      );
    } finally {
      setBusy(false);
    }
  };

  const command = code ? `agpj connect ${code.code}` : '';
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
    } catch {
      setError(t('agentConnection.copyFailed'));
    }
  };

  return (
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent
        className="modal"
        showCloseButton={false}
        aria-describedby={undefined}
        onCloseAutoFocus={(event) => {
          const target = restoreFocusRef?.current;
          if (!target?.isConnected) return;
          event.preventDefault();
          target.focus();
        }}
      >
        <div className="modal__header">
          <DialogTitle className="modal__title">{t('agentConnection.title')}</DialogTitle>
          <button
            type="button"
            className="modal__close"
            aria-label={t('dialog.close')}
            onClick={() => onOpenChange(false)}
          >
            ×
          </button>
        </div>
        {loading ? <p>{t('agentConnection.loading')}</p> : null}
        {!loading && !agentId && !query.isError ? <p>{t('agentConnection.noAgent')}</p> : null}
        {agentId && !code ? (
          <Button type="button" disabled={busy} onClick={() => void issue()}>
            {t('agentConnection.issue')}
          </Button>
        ) : null}
        {code ? (
          <>
            <p>
              {t('agentConnection.expires', {
                time: new Date(code.expiresAt).toLocaleTimeString(),
              })}
            </p>
            <code>{command}</code>
            <Button type="button" onClick={() => void copy()}>
              {copied ? t('agentConnection.copied') : t('agentConnection.copy')}
            </Button>
          </>
        ) : null}
        {projectId && !loading && !query.isError ? (
          <AgentList
            agents={agents}
            busy={writing}
            onChangeTool={(id, tool) => changeTool.mutate({ agentId: id, tool })}
            onRegister={(input) => register.mutate(input)}
          />
        ) : null}
        {error || query.isError || writeFailed ? (
          <p className="modal__error" role="alert">
            {error ||
              (query.isError ? t('agentConnection.loadFailed') : t('agentConnection.saveFailed'))}
          </p>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
