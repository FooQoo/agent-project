import { RadioGroupItem } from '@/shared/ui/radio-group';
import { AgentIdentity } from './AgentParts';
import { type MockAgent, type MockCard, loadOf } from './data';
import { useMockText } from './text';
import './assignment-mock.css';

/** One agent as a choice: the same row size for every agent, with state and load beside the name. */
export function AgentChoice({
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
