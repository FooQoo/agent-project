import { AlertTriangleIcon, ArrowRightIcon, CopyIcon, MonitorIcon, PlusIcon } from 'lucide-react';
import { useId, useState } from 'react';
import { Button } from '@/shared/ui/button';
import { Checkbox } from '@/shared/ui/checkbox';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { Input } from '@/shared/ui/input';
import { RadioGroup, RadioGroupItem } from '@/shared/ui/radio-group';
import { AgentChip, AgentIdentity, Avatar, SessionLine, StateBadge } from './AgentParts';
import { type Fixture, type MockAgent, type MockCard, agentOf, loadOf } from './data';
import { Surface } from './Surface';
import { useMockText } from './text';
import './assignment-mock.css';

type Kind = 'dialog' | 'sheet';

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

/**
 * Switching the parent session of an agent: the target and what happens to the current session come first,
 * and only then the one-time code appears (SPEC 6.1). The code is shown on the PC screen only.
 */
export function SwitchSurface({
  kind,
  agent,
  cards,
  showCode,
  onShowCode,
  onClose,
}: {
  kind: Kind;
  agent: MockAgent;
  cards: readonly MockCard[];
  showCode: boolean;
  onShowCode: () => void;
  onClose: () => void;
}) {
  const m = useMockText();
  const running = cards.filter((card) => card.agentId === agent.id && card.running).length;
  return (
    <Surface
      kind={kind}
      title={showCode ? m.switchCodeTitle : m.switchTitle}
      onClose={onClose}
      className="aswitch"
      footer={
        <DialogActions>
          <Button type="button" variant="outline" onClick={onClose}>
            {showCode ? m.close : m.cancel}
          </Button>
          {!showCode && (
            <Button type="button" onClick={onShowCode}>
              {m.switchShowCode}
            </Button>
          )}
        </DialogActions>
      }
    >
      <div className="adialog">
        <section>
          <h3 className="adialog__h">{m.switchTarget}</h3>
          <div className="atarget">
            <Avatar agent={agent} />
            <div className="atarget__text">
              <strong>{agent.name}</strong>
              <span>{agent.role}</span>
            </div>
          </div>
        </section>
        <div className="aswap" role="group" aria-label={`${m.switchNow} / ${m.switchAfter}`}>
          <div className="aswap__box">
            <span className="aswap__head">{m.switchNow}</span>
            {agent.session ? <SessionLine session={agent.session} /> : <span>{m.noSession}</span>}
            <StateBadge state={agent.state} />
          </div>
          <ArrowRightIcon aria-hidden="true" className="aswap__arrow" />
          <div className="aswap__box" data-after="true">
            <span className="aswap__head">{m.switchAfter}</span>
            <span>{m.switchAfterBody}</span>
          </div>
        </div>
        {showCode ? (
          <div className="acode">
            <p>{m.switchCodeBody}</p>
            <div className="acode__row">
              <code className="acode__cmd">agpj connect K7QM-4XD9</code>
              <Button type="button" size="s" variant="outline">
                <CopyIcon aria-hidden="true" />
              </Button>
            </div>
            <span className="acode__meta">{m.switchCodeExpires}</span>
            <p className="acode__wait">{m.switchCodeWait}</p>
          </div>
        ) : (
          <ul className="aeffects">
            {agent.session && <li>{m.switchEffect1}</li>}
            {running > 0 && <li>{m.switchEffect2(running)}</li>}
            <li>{m.switchEffect3}</li>
          </ul>
        )}
      </div>
    </Surface>
  );
}

/** One agent as a choice: the same row size for every agent, with state and load beside the name. */
function AgentChoice({
  agent,
  cards,
  current,
  group,
}: {
  agent: MockAgent;
  cards: readonly MockCard[];
  current: boolean;
  group: string;
}) {
  const m = useMockText();
  const id = `${group}-${agent.id}`;
  return (
    <label htmlFor={id} className="achoice" data-current={current || undefined}>
      <RadioGroupItem id={id} value={agent.id} disabled={current} />
      <AgentIdentity agent={agent} showSession={false} />
      <span className="achoice__side">
        {current ? (
          <span className="achoice__current">{m.reassignCurrent}</span>
        ) : (
          <span>{m.reassignLoad(loadOf(cards, agent.id))}</span>
        )}
      </span>
    </label>
  );
}

/**
 * Handing a card to another agent in the middle of the work. It names both agents, lists what carries over
 * and what stops, and asks for one more confirmation when the new agent has no parent session.
 */
export function ReassignSurface({
  kind,
  fixture,
  cards,
  card,
  initialTo,
  onClose,
  onConfirm,
}: {
  kind: Kind;
  fixture: Fixture;
  cards: readonly MockCard[];
  card: MockCard;
  initialTo?: string;
  onClose: () => void;
  onConfirm: (agentId: string) => void;
}) {
  const m = useMockText();
  const group = useId();
  const ackId = useId();
  const [to, setTo] = useState<string | null>(initialTo ?? null);
  const [ack, setAck] = useState(false);
  const from = agentOf(fixture.agents, card.agentId);
  const target = agentOf(fixture.agents, to);
  const needAck = target !== null && target.state !== 'connected';
  const laneName = m[card.lane];
  return (
    <Surface
      kind={kind}
      title={m.reassignTitle}
      onClose={onClose}
      className="areassign"
      footer={
        <>
          {!target && <p className="aneed">{m.reassignPick}</p>}
          {target && needAck && !ack && <p className="aneed">{m.reassignNeedAck}</p>}
          <DialogActions>
            <Button type="button" variant="outline" onClick={onClose}>
              {m.cancel}
            </Button>
            <Button
              type="button"
              disabled={!target || (needAck && !ack)}
              onClick={() => target && onConfirm(target.id)}
            >
              {target ? m.reassignConfirm(target.name) : m.reassignTitle}
            </Button>
          </DialogActions>
        </>
      }
    >
      <div className="adialog">
        <p className="adialog__scope">{m.reassignScope(card.id, card.title)}</p>
        <div className="aswap" role="group" aria-label={`${m.reassignFrom} / ${m.reassignTo}`}>
          <div className="aswap__box">
            <span className="aswap__head">{m.reassignFrom}</span>
            <AgentChip agent={from} />
          </div>
          <ArrowRightIcon aria-hidden="true" className="aswap__arrow" />
          <div className="aswap__box" data-after="true">
            <span className="aswap__head">{m.reassignTo}</span>
            <AgentChip agent={target} />
          </div>
        </div>
        <RadioGroup
          value={to ?? ''}
          onValueChange={(value) => {
            setTo(value);
            setAck(false);
          }}
          className="achoices"
          aria-label={m.reassignTo}
        >
          {fixture.agents.map((agent) => (
            <AgentChoice
              key={agent.id}
              agent={agent}
              cards={cards}
              current={agent.id === card.agentId}
              group={group}
            />
          ))}
        </RadioGroup>
        <div className="acarry">
          <section>
            <h3 className="adialog__h">{m.reassignCarry}</h3>
            <ul className="aeffects">
              <li>{m.reassignCarryLane(laneName)}</li>
              <li>{m.reassignCarryPlan}</li>
              <li>{m.reassignCarryResults(card.results)}</li>
              {card.id === 28 && <li>{m.reassignCarryQuestion}</li>}
            </ul>
          </section>
          {from && (
            <section>
              <h3 className="adialog__h">{m.reassignStops}</h3>
              <p className="acarry__stop">{m.reassignStopsBody(from.name)}</p>
            </section>
          )}
        </div>
        {target && needAck && (
          <div className="awarn" role="alert">
            <AlertTriangleIcon aria-hidden="true" />
            <div>
              <strong>{m.reassignWarnTitle}</strong>
              <p>{m.reassignWarnBody}</p>
              <div className="awarn__ack">
                <Checkbox
                  id={ackId}
                  checked={ack}
                  onCheckedChange={(value) => setAck(value === true)}
                />
                <label htmlFor={ackId}>{m.reassignWarnAck}</label>
              </div>
            </div>
          </div>
        )}
      </div>
    </Surface>
  );
}

/**
 * The bulk action on an epic: it targets only unassigned cards, lists the ones that change and the ones that
 * stay (with the reason), and shows the agent's open-card count before and after.
 */
export function BulkSurface({
  kind,
  fixture,
  cards,
  epicId,
  initialTo,
  onClose,
  onConfirm,
}: {
  kind: Kind;
  fixture: Fixture;
  cards: readonly MockCard[];
  epicId: string;
  initialTo?: string;
  onClose: () => void;
  onConfirm: (agentId: string) => void;
}) {
  const m = useMockText();
  const group = useId();
  const [to, setTo] = useState<string | null>(initialTo ?? null);
  const epic = fixture.epics.find((item) => item.id === epicId);
  const inEpic = cards.filter((card) => card.epicId === epicId);
  const will = inEpic.filter((card) => card.agentId === null);
  const skip = inEpic.filter((card) => card.agentId !== null);
  const target = agentOf(fixture.agents, to);
  const before = target ? loadOf(cards, target.id) : 0;
  return (
    <Surface
      kind={kind}
      title={m.bulkTitle(epic?.name ?? '')}
      onClose={onClose}
      className="abulk"
      footer={
        <DialogActions>
          <Button type="button" variant="outline" onClick={onClose}>
            {m.cancel}
          </Button>
          <Button
            type="button"
            disabled={!target || will.length === 0}
            onClick={() => target && onConfirm(target.id)}
          >
            {target ? m.bulkConfirm(will.length, target.name) : m.reassignPick}
          </Button>
        </DialogActions>
      }
    >
      <div className="adialog">
        <p className="adialog__lead">{m.bulkLead}</p>
        <section>
          <h3 className="adialog__h">{m.bulkAgent}</h3>
          <RadioGroup
            value={to ?? ''}
            onValueChange={setTo}
            className="achoices"
            aria-label={m.bulkAgent}
          >
            {fixture.agents.map((agent) => (
              <AgentChoice
                key={agent.id}
                agent={agent}
                cards={cards}
                current={false}
                group={group}
              />
            ))}
          </RadioGroup>
        </section>
        <section>
          <h3 className="adialog__h">{m.bulkWill(will.length)}</h3>
          {will.length === 0 ? (
            <p className="adialog__lead">{m.bulkNone}</p>
          ) : (
            <ul className="alines">
              {will.map((card) => (
                <li key={card.id} className="alines__item" data-change="true">
                  <span className="alines__id">#{card.id}</span>
                  <span className="alines__title">{card.title}</span>
                  <span className="alines__lane">{m[card.lane]}</span>
                  <AgentChip agent={target} />
                </li>
              ))}
            </ul>
          )}
          <p className="alines__note">
            {m.bulkLaneKept} ・ {m.bulkLater}
          </p>
        </section>
        {target && (
          <p className="abefore">{m.bulkAfter(target.name, before, before + will.length)}</p>
        )}
        <section>
          <h3 className="adialog__h">{m.bulkSkip(skip.length)}</h3>
          <ul className="alines">
            {skip.map((card) => {
              const holder = agentOf(fixture.agents, card.agentId);
              return (
                <li key={card.id} className="alines__item">
                  <span className="alines__id">#{card.id}</span>
                  <span className="alines__title">{card.title}</span>
                  <span className="alines__lane">{m[card.lane]}</span>
                  <span className="alines__why">
                    {card.lane === 'done' ? m.bulkSkipDone : m.bulkSkipAssigned(holder?.name ?? '')}
                  </span>
                </li>
              );
            })}
          </ul>
        </section>
      </div>
    </Surface>
  );
}
