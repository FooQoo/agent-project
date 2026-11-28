import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../shared/ui/button';
import { Input } from '../../shared/ui/input';
import { AGENT_TOOLS, type Agent, type AgentTool, type NewAgent } from './api';
import './agent-list.css';

const toolValue = (tool: AgentTool | null): string => tool ?? '';

function ToolSelect({
  agent,
  disabled,
  onChange,
}: {
  agent: Agent;
  disabled: boolean;
  onChange: (tool: AgentTool) => void;
}) {
  const { t } = useTranslation();
  return (
    <select
      className="agent-list__tool"
      aria-label={t('agentConnection.toolFor', { name: agent.name })}
      value={toolValue(agent.tool)}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value as AgentTool)}
    >
      {agent.tool === null ? <option value="">{t('agentConnection.toolNone')}</option> : null}
      {AGENT_TOOLS.map((tool) => (
        <option key={tool} value={tool}>
          {t(`agentConnection.tool.${tool}`)}
        </option>
      ))}
    </select>
  );
}

/** The project's agents with each one's tool, and a form that registers another agent. */
export function AgentList({
  agents,
  busy,
  onChangeTool,
  onRegister,
}: {
  agents: Agent[];
  busy: boolean;
  onChangeTool: (agentId: string, tool: AgentTool) => void;
  onRegister: (input: NewAgent) => void;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState('');
  const [role, setRole] = useState('');
  const [tool, setTool] = useState<AgentTool | null>(null);
  const submit = () => {
    if (name.trim() === '' || busy) return;
    onRegister({ name: name.trim(), role: role.trim(), tool });
    setName('');
    setRole('');
    setTool(null);
  };
  return (
    <section className="agent-list" aria-label={t('agentConnection.agents')}>
      <h3 className="agent-list__title">{t('agentConnection.agents')}</h3>
      <ul className="agent-list__rows">
        {agents.map((agent) => (
          <li key={agent.id} className="agent-list__row">
            <span className="agent-list__who">
              <span className="agent-list__name">{agent.name}</span>
              {agent.role ? <span className="agent-list__role">{agent.role}</span> : null}
            </span>
            <ToolSelect
              agent={agent}
              disabled={busy}
              onChange={(next) => onChangeTool(agent.id, next)}
            />
          </li>
        ))}
      </ul>
      <form
        className="agent-list__form"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <Input
          value={name}
          maxLength={50}
          placeholder={t('agentConnection.namePlaceholder')}
          aria-label={t('agentConnection.name')}
          onChange={(event) => setName(event.target.value)}
        />
        <Input
          value={role}
          maxLength={100}
          placeholder={t('agentConnection.rolePlaceholder')}
          aria-label={t('agentConnection.role')}
          onChange={(event) => setRole(event.target.value)}
        />
        <select
          className="agent-list__tool"
          aria-label={t('agentConnection.tool.label')}
          value={toolValue(tool)}
          onChange={(event) => setTool((event.target.value || null) as AgentTool | null)}
        >
          <option value="">{t('agentConnection.toolNone')}</option>
          {AGENT_TOOLS.map((value) => (
            <option key={value} value={value}>
              {t(`agentConnection.tool.${value}`)}
            </option>
          ))}
        </select>
        <Button type="submit" disabled={busy || name.trim() === ''}>
          {t('agentConnection.register')}
        </Button>
      </form>
    </section>
  );
}
