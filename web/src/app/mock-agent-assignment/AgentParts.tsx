import { CircleDashedIcon, LinkIcon, ReplaceIcon, UserCogIcon } from 'lucide-react';
import type { CSSProperties, ReactNode } from 'react';
import { Button } from '@/shared/ui/button';
import {
  type ConnectionState,
  type MockAgent,
  type MockCard,
  type MockSession,
  loadOf,
  unassignedIn,
} from './data';
import { type MockText, useMockText } from './text';
import './assignment-mock.css';

const stateLabel = (m: MockText, state: ConnectionState) =>
  state === 'connected'
    ? m.stateConnected
    : state === 'unconnected'
      ? m.stateUnconnected
      : m.stateReplaced;

/** The agent's initial in its own color. The name always stays beside it, so color is never the only cue. */
export function Avatar({ agent }: { agent: MockAgent }) {
  return (
    <span
      className="aavatar"
      style={{ '--agent': agent.color } as CSSProperties}
      aria-hidden="true"
    >
      {Array.from(agent.name)[0]}
    </span>
  );
}

/** Connected, not connected, replaced: icon and text, never color alone. */
export function StateBadge({ state }: { state: ConnectionState }) {
  const m = useMockText();
  const Icon =
    state === 'connected' ? LinkIcon : state === 'unconnected' ? CircleDashedIcon : ReplaceIcon;
  return (
    <span className="astate" data-state={state}>
      <Icon aria-hidden="true" />
      {stateLabel(m, state)}
    </span>
  );
}

/** The agent on a card: initial and name. A card with no agent shows a dashed "Unassigned". */
export function AgentChip({ agent, mark }: { agent: MockAgent | null; mark?: string }) {
  const m = useMockText();
  if (!agent) return <span className="achip achip--none">{m.unassigned}</span>;
  return (
    <span className="achip" data-agent={agent.id} data-state={agent.state}>
      <Avatar agent={agent} />
      <span className="achip__name">{agent.name}</span>
      {mark && <span className="achip__mark">{mark}</span>}
    </span>
  );
}

export function SessionLine({ session, tone }: { session: MockSession; tone?: 'old' }) {
  const m = useMockText();
  return (
    <span className="asession" data-tone={tone}>
      <strong>{m.sessionOf(session.tool, session.label)}</strong>
      <span>{m.idle(session.idleMin)}</span>
    </span>
  );
}

/** What the strip, the dialogs and the detail all show for one agent: who, what role, which session. */
export function AgentIdentity({
  agent,
  extra,
  showSession = true,
}: {
  agent: MockAgent;
  extra?: ReactNode;
  showSession?: boolean;
}) {
  const m = useMockText();
  return (
    <div className="aident">
      <Avatar agent={agent} />
      <div className="aident__text">
        <div className="aident__head">
          <strong className="aident__name">{agent.name}</strong>
          <StateBadge state={agent.state} />
        </div>
        <span className="aident__role">{agent.role}</span>
        {showSession &&
          (agent.session ? (
            <SessionLine session={agent.session} />
          ) : (
            <span className="asession asession--none">{m.noSession}</span>
          ))}
        {extra}
      </div>
    </div>
  );
}

/** The row under the header: the agents with their state and open cards, and the epics with unassigned cards. */
export function AgentStrip({
  agents,
  cards,
  epics,
  onManage,
  onEpic,
}: {
  agents: readonly MockAgent[];
  cards: readonly MockCard[];
  epics: readonly { id: string; name: string }[];
  onManage: () => void;
  onEpic: (epicId: string) => void;
}) {
  const m = useMockText();
  return (
    <div className="astrip" role="region" aria-label={m.stripLabel}>
      <ul className="astrip__agents">
        {agents.map((agent) => (
          <li key={agent.id} className="astrip__agent" data-state={agent.state}>
            <Avatar agent={agent} />
            <span className="astrip__name">{agent.name}</span>
            <span
              className="astrip__dot"
              data-state={agent.state}
              title={stateLabel(m, agent.state)}
            />
            <span className="astrip__state">{stateLabel(m, agent.state)}</span>
            <span className="astrip__count">{m.cardsOf(loadOf(cards, agent.id))}</span>
          </li>
        ))}
      </ul>
      <div className="astrip__tools">
        {epics.map((epic) => {
          const n = unassignedIn(cards, epic.id).length;
          return (
            n > 0 && (
              <Button
                key={epic.id}
                type="button"
                size="s"
                variant="outline"
                onClick={() => onEpic(epic.id)}
              >
                {m.epicUnassigned(epic.name, n)}
                <span className="astrip__go">{m.epicAssign}</span>
              </Button>
            )
          );
        })}
        <Button type="button" size="s" variant="outline" onClick={onManage}>
          <UserCogIcon aria-hidden="true" />
          {m.stripManage}
        </Button>
      </div>
    </div>
  );
}

/** The assignment block above the card detail: who holds the card and the way to change it. */
export function AssignSection({
  card,
  agent,
  onChange,
}: {
  card: MockCard;
  agent: MockAgent | null;
  onChange: () => void;
}) {
  const m = useMockText();
  const finished = card.lane === 'done';
  return (
    <section className="asection" aria-label={m.assignSection}>
      <h3 className="asection__h">{m.assignSection}</h3>
      {agent ? (
        <AgentIdentity
          agent={agent}
          extra={
            <span className="asection__meta">
              {m.assignGeneration(card.generation)}
              {card.running && ` ・ ${m.assignRunning}`}
            </span>
          }
        />
      ) : (
        <p className="asection__none">{m.unassigned}</p>
      )}
      {finished ? (
        <p className="asection__note">{m.assignDoneNote}</p>
      ) : (
        <Button type="button" size="s" variant="outline" onClick={onChange}>
          {agent ? m.assignChange : m.assignAssign}
        </Button>
      )}
    </section>
  );
}
