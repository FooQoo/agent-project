import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../shared/ui/button';
import type { Agent } from '../../shared/api/agents';
import { issueConnectCode, type ConnectCode } from './api';

export function AgentCodePanel({
  projectId,
  agent,
  onClose,
}: {
  projectId: string;
  agent: Agent;
  onClose(): void;
}) {
  const { t } = useTranslation();
  const [code, setCode] = useState<ConnectCode | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);

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
    if (busy) return;
    setBusy(true);
    setError('');
    try {
      setCode(await issueConnectCode(projectId, agent.id, agent.connection !== null));
    } catch {
      setError(t('agentConnection.issueFailed'));
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
    <div className="agent-settings__code">
      {code ? (
        <>
          <p>
            {t('agentConnection.expires', { time: new Date(code.expiresAt).toLocaleTimeString() })}
          </p>
          <code>{command}</code>
        </>
      ) : agent.connection ? (
        <p>{t('agentConnection.replaceHelp')}</p>
      ) : null}
      {error && <p role="alert">{error}</p>}
      <div className="agent-settings__actions">
        <Button type="button" variant="outline" onClick={onClose}>
          {t('agentConnection.close')}
        </Button>
        {code ? (
          <Button type="button" onClick={() => void copy()}>
            {copied ? t('agentConnection.copied') : t('agentConnection.copy')}
          </Button>
        ) : (
          <Button type="button" disabled={busy} onClick={() => void issue()}>
            {t(agent.connection ? 'agentConnection.replace' : 'agentConnection.issue')}
          </Button>
        )}
      </div>
    </div>
  );
}
