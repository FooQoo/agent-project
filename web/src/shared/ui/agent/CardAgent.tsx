import type { AssignedAgent } from '../../api/contracts';
import { AgentAvatar } from './AgentIdentity';

export function CardAgent({ agent }: { agent: AssignedAgent }) {
  return (
    <span className="card-agent" role="img" title={agent.name} aria-label={agent.name}>
      <AgentAvatar {...agent} />
    </span>
  );
}
