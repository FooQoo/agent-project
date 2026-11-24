import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import type { Agent } from '../../api/agents';
import './agent.css';

export function AgentAvatar({ id, name }: Pick<Agent, 'id' | 'name'>) {
  const number = Number(id.slice(2));
  const hue = Number.isFinite(number) ? (number * 137.5 + 85) % 360 : 220;
  return (
    <span
      aria-hidden="true"
      className="agent-avatar"
      style={{ '--agent-hue': hue } as CSSProperties}
    >
      {Array.from(name)[0]}
    </span>
  );
}

export function AgentIdentity({ agent }: { agent: Agent }) {
  const { t } = useTranslation();
  return (
    <span className="agent-identity">
      <AgentAvatar {...agent} />
      <span className="agent-identity__text">
        <strong>{agent.name}</strong>
        <span>{agent.role}</span>
      </span>
      <span className="agent-identity__status">
        {t(agent.connection ? 'agentConnection.connected' : 'agentConnection.disconnected')}
      </span>
    </span>
  );
}
