import { CircleDashedIcon, LinkIcon, ReplaceIcon } from 'lucide-react';
import { type CSSProperties, type ReactNode, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Button } from '@/shared/ui/button';
import type { ConnectionState, MockAgent, MockSession } from './data';
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

/**
 * Puts its children inside a row the product's component draws, as the last item before the row's spacer
 * (or at the end of the row). The product components have no slot for the mock's avatar, so the mock
 * adds one into the real row instead of copying the component.
 */
export function RowSlot({
  row,
  beforeSelector,
  className,
  children,
}: {
  /** Finds the row from the card or the detail that holds the anchor. */
  row: string;
  beforeSelector?: string;
  className?: string;
  children: ReactNode;
}) {
  const anchor = useRef<HTMLSpanElement>(null);
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  useLayoutEffect(() => {
    const host = anchor.current?.parentElement?.querySelector(row);
    if (!host) return;
    const el = document.createElement('span');
    el.className = className ?? '';
    host.insertBefore(el, (beforeSelector && host.querySelector(beforeSelector)) || null);
    setSlot(el);
    return () => el.remove();
  }, [row, beforeSelector, className]);
  return (
    <>
      <span hidden ref={anchor} />
      {slot && createPortal(children, slot)}
    </>
  );
}

/** The agent's initial and name as a chip, for the dialogs that name an agent in a sentence. */
export function AgentChip({ agent }: { agent: MockAgent | null }) {
  const m = useMockText();
  if (!agent) return <span className="achip achip--none">{m.unassigned}</span>;
  return (
    <span className="achip" data-agent={agent.id} data-state={agent.state}>
      <Avatar agent={agent} />
      <span className="achip__name">{agent.name}</span>
    </span>
  );
}

/**
 * The agent on a card: its small avatar and short name in the card's meta row. A card with no agent
 * shows nothing, and the connection state is left to Settings and the chooser.
 */
export function CardAgent({ agent }: { agent: MockAgent | null }) {
  if (!agent) return null;
  return (
    <RowSlot row=".card__top" beforeSelector=".card__spacer" className="acardagent">
      <span className="acardagent__chip" data-agent={agent.id}>
        <Avatar agent={agent} />
        <span className="acardagent__name">{agent.name}</span>
      </span>
    </RowSlot>
  );
}

/** The detail's only assignment control: the assignee's avatar in the status row, which opens the chooser. */
export function AssigneeButton({
  agent,
  disabled,
  onChange,
}: {
  agent: MockAgent | null;
  disabled: boolean;
  onChange: () => void;
}) {
  const m = useMockText();
  if (!agent) return null;
  return (
    <RowSlot row=".detail__meta" className="aassignee">
      <button
        type="button"
        className="aassignee__btn"
        disabled={disabled}
        aria-label={`${m.assignChange}: ${agent.name}`}
        title={agent.name}
        onClick={onChange}
      >
        <Avatar agent={agent} />
      </button>
    </RowSlot>
  );
}

/** On the epic tab: one small button that opens the bulk change dialog (no extra box or text). */
export function EpicAssignBar({ count, onAssign }: { count: number; onAssign: () => void }) {
  const m = useMockText();
  if (count === 0) return null;
  return (
    <Button type="button" size="s" variant="outline" className="aepicbtn" onClick={onAssign}>
      {m.epicChangeAll}
    </Button>
  );
}
