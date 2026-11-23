import { useId, useState } from 'react';
import { Button } from '@/shared/ui/button';
import { DialogActions } from '@/shared/ui/dialog-actions';
import { RadioGroup } from '@/shared/ui/radio-group';
import { AgentChip } from './AgentParts';
import { AgentChoice } from './AgentChoice';
import { type Fixture, type MockCard, agentOf, loadOf } from './data';
import { Surface } from './Surface';
import { useMockText } from './text';
import type { Kind } from './surface-kind';
import './assignment-mock.css';

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
