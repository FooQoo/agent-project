import { Link2Icon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { SettingsEntryRow } from '../../shared/ui/settings-entry-row';

export function AgentConnectionEntryRow({ onOpen }: { onOpen: () => void }) {
  const { t } = useTranslation();
  return (
    <SettingsEntryRow
      icon={Link2Icon}
      title={t('agentConnection.entryTitle')}
      body={t('agentConnection.entryBody')}
      action={t('agentConnection.entryAction')}
      onOpen={onOpen}
    />
  );
}
