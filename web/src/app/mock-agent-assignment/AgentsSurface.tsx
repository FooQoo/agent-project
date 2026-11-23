import { MonitorIcon, PlusIcon } from 'lucide-react';
import { useId, useState } from 'react';
import { Button } from '@/shared/ui/button';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { Input } from '@/shared/ui/input';
import { AgentIdentity, SessionLine, StateBadge } from './AgentParts';
import { type MockAgent, type MockCard, loadOf } from './data';
import { Surface } from './Surface';
import { useMockText } from './text';
import type { Kind } from './surface-kind';
import './assignment-mock.css';

/** Settings > Agents: who the agents are, which parent session each one has, and how to switch it. */
export function AgentsSurface({
  kind,
  agents,
  cards,
  adding,
  onSwitch,
  onClose,
}: {
  kind: Kind;
  agents: readonly MockAgent[];
  cards: readonly MockCard[];
  adding?: boolean;
  onSwitch: (agentId: string) => void;
  onClose: () => void;
}) {
  const m = useMockText();
  const [form, setForm] = useState(adding ?? false);
  const phone = kind === 'sheet';
  return (
    <Surface
      kind={kind}
      title={m.agentsTitle}
      onClose={onClose}
      className="aagents"
      footer={
        <DialogActions>
          <Button type="button" variant="outline" onClick={onClose}>
            {m.close}
          </Button>
        </DialogActions>
      }
    >
      <div className="adialog">
        <p className="adialog__lead">{m.agentsLead}</p>
        {form ? (
          <AgentForm onCancel={() => setForm(false)} />
        ) : (
          <Button
            type="button"
            size="s"
            variant="outline"
            className="adialog__add"
            onClick={() => setForm(true)}
          >
            <PlusIcon aria-hidden="true" />
            {m.agentAdd}
          </Button>
        )}
        <ul className="alist">
          {agents.map((agent) => (
            <li key={agent.id} className="alist__item" data-state={agent.state}>
              <AgentIdentity
                agent={agent}
                extra={
                  <>
                    <span className="alist__meta">
                      {agent.id} ・ {m.cardsOf(loadOf(cards, agent.id))}
                    </span>
                    {agent.replacedSession && (
                      <span className="alist__prev">
                        <span className="alist__prev-head">
                          {m.previousSession}
                          <StateBadge state="replaced" />
                        </span>
                        <SessionLine session={agent.replacedSession} tone="old" />
                        <span className="alist__prev-note">{m.replacedNote}</span>
                      </span>
                    )}
                  </>
                }
              />
              <div className="alist__actions">
                <Button type="button" size="s" variant="outline">
                  {m.agentEdit}
                </Button>
                {phone ? (
                  <p className="alist__pc">
                    <MonitorIcon aria-hidden="true" />
                    {m.agentPcOnly}
                  </p>
                ) : (
                  <Button
                    type="button"
                    size="s"
                    variant="outline"
                    onClick={() => onSwitch(agent.id)}
                  >
                    {agent.state === 'connected' ? m.agentSwitch : m.agentCode}
                  </Button>
                )}
              </div>
            </li>
          ))}
        </ul>
      </div>
    </Surface>
  );
}

function AgentForm({ onCancel }: { onCancel: () => void }) {
  const m = useMockText();
  const nameId = useId();
  const roleId = useId();
  const [name, setName] = useState('');
  const [role, setRole] = useState('');
  return (
    <form
      className="aform"
      onSubmit={(event) => {
        event.preventDefault();
        onCancel();
      }}
    >
      <label htmlFor={nameId}>{m.agentName}</label>
      <Input id={nameId} value={name} maxLength={50} onChange={(e) => setName(e.target.value)} />
      <span className="aform__hint">{m.agentNameHint}</span>
      <label htmlFor={roleId}>{m.agentRole}</label>
      <Input id={roleId} value={role} maxLength={100} onChange={(e) => setRole(e.target.value)} />
      <span className="aform__hint">{m.agentRoleHint}</span>
      <DialogActions size="s" align="start">
        <Button type="button" variant="outline" onClick={onCancel}>
          {m.cancel}
        </Button>
        <Button type="submit" disabled={name.trim() === ''}>
          {m.agentSave}
        </Button>
      </DialogActions>
    </form>
  );
}
