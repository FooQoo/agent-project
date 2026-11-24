import { useTranslation } from 'react-i18next';
import type { AssignedAgent } from '../../api/contracts';
import { AgentAvatar } from './AgentIdentity';

export function CardAgent({ agent }: { agent: AssignedAgent }) {
  const { t } = useTranslation();
  return (
    <span className="card-agent" role="img" title={agent.name} aria-label={t('assignee.ai')}>
      <AgentAvatar {...agent} />
    </span>
  );
}
