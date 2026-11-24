import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../shared/ui/button';
import { Input } from '../../shared/ui/input';
import type { Agent } from '../../shared/api/agents';

export function AgentRegistrationForm({
  agent,
  onSave,
  onClose,
}: {
  agent?: Agent;
  onSave(value: Pick<Agent, 'name' | 'role'>): Promise<void>;
  onClose(): void;
}) {
  const { t } = useTranslation();
  const nameId = useId();
  const roleId = useId();
  const [name, setName] = useState(agent?.name ?? '');
  const [role, setRole] = useState(agent?.role ?? '');
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);

  const save = async () => {
    if (saving || !name.trim()) return;
    setSaving(true);
    setFailed(false);
    try {
      await onSave({ name: name.trim(), role: role.trim() });
      onClose();
    } catch {
      setFailed(true);
    } finally {
      setSaving(false);
    }
  };

  return (
    <form
      className="agent-settings__form"
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <label htmlFor={nameId}>{t('agentConnection.name')}</label>
      <Input
        id={nameId}
        value={name}
        onChange={(event) => setName(event.target.value)}
        maxLength={50}
        required
      />
      <label htmlFor={roleId}>{t('agentConnection.role')}</label>
      <Input
        id={roleId}
        value={role}
        onChange={(event) => setRole(event.target.value)}
        maxLength={100}
      />
      {failed && <p role="alert">{t('agentConnection.saveFailed')}</p>}
      <div className="agent-settings__actions">
        <Button type="button" variant="outline" disabled={saving} onClick={onClose}>
          {t('agentConnection.cancel')}
        </Button>
        <Button type="submit" disabled={saving || !name.trim()}>
          {t('agentConnection.save')}
        </Button>
      </div>
    </form>
  );
}
