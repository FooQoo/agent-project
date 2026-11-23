import { AlertTriangleIcon, ArrowRightIcon } from 'lucide-react';
import { useId, useState } from 'react';
import { Button } from '@/shared/ui/button';
import { Checkbox } from '@/shared/ui/checkbox';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { RadioGroup } from '@/shared/ui/radio-group';
import { AgentChip } from './AgentParts';
import { AgentChoice } from './AgentChoice';
import { type Fixture, type MockCard, agentOf } from './data';
import { Surface } from './Surface';
import { useMockText } from './text';
import type { Kind } from './surface-kind';
import './assignment-mock.css';

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
