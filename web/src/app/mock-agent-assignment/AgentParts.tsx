import { CheckIcon, ChevronDownIcon, CircleDashedIcon, LinkIcon } from 'lucide-react';
import { type CSSProperties, type ReactNode, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/shared/ui/dropdown-menu';
import { Surface } from './Surface';
import type { ConnectionState, MockAgent, MockSession } from './data';
import { type MockText, useMockText } from './text';
import './assignment-mock.css';

const stateLabel = (m: MockText, state: ConnectionState) =>
  state === 'connected' ? m.stateConnected : m.stateUnconnected;

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

/** Connected or not connected: icon and text, never color alone. */
export function StateBadge({ state }: { state: ConnectionState }) {
  const m = useMockText();
  const Icon = state === 'connected' ? LinkIcon : CircleDashedIcon;
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

/**
 * The detail's only assignment control: the assignee (avatar, name, chevron) in the status row. On the PC it
 * opens a menu under it; on a phone it opens a bottom sheet with the same list. Choosing an agent changes the
 * assignee at once.
 */
export function AssigneeMenu({
  agent,
  agents,
  disabled,
  phone,
  defaultOpen = false,
  onSelect,
}: {
  agent: MockAgent | null;
  agents: readonly MockAgent[];
  disabled: boolean;
  phone: boolean;
  defaultOpen?: boolean;
  onSelect: (agentId: string) => void;
}) {
  const m = useMockText();
  const [sheet, setSheet] = useState(defaultOpen && phone);
  if (!agent) return null;
  const trigger = (
    <button
      type="button"
      className="aassignee__btn"
      disabled={disabled}
      aria-label={`${m.assignChange}: ${agent.name}`}
      onClick={phone ? () => setSheet(true) : undefined}
    >
      <Avatar agent={agent} />
      <span className="aassignee__name">{agent.name}</span>
      <ChevronDownIcon aria-hidden="true" />
    </button>
  );
  const row = (item: MockAgent) => (
    <>
      <Avatar agent={item} />
      <span className="aassignee__item-name">{item.name}</span>
      <StateBadge state={item.state} />
    </>
  );
  return (
    <RowSlot row=".detail__meta" className="aassignee">
      {phone ? (
        <>
          {trigger}
          {sheet && (
            <Surface kind="sheet" title={m.reassignTitle} onClose={() => setSheet(false)}>
              <ul className="aassignee__list">
                {agents.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      className="aassignee__row"
                      data-current={item.id === agent.id || undefined}
                      data-agent-state={item.state}
                      onClick={() => {
                        setSheet(false);
                        if (item.id !== agent.id) onSelect(item.id);
                      }}
                    >
                      {row(item)}
                      {item.id === agent.id && <CheckIcon aria-hidden="true" />}
                    </button>
                  </li>
                ))}
              </ul>
            </Surface>
          )}
        </>
      ) : (
        <DropdownMenu defaultOpen={defaultOpen}>
          <DropdownMenuTrigger asChild>{trigger}</DropdownMenuTrigger>
          <DropdownMenuContent className="aassignee__menu">
            <DropdownMenuRadioGroup
              value={agent.id}
              onValueChange={(id) => id !== agent.id && onSelect(id)}
            >
              {agents.map((item) => (
                <DropdownMenuRadioItem key={item.id} value={item.id} data-agent-state={item.state}>
                  {row(item)}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </RowSlot>
  );
}
