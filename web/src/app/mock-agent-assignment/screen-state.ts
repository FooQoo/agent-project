import type { DetailTab } from '../../features/tasks/detail/DetailTabs';
import { type Lane, type MockCard, buildFixture } from './data';

export type DialogName = 'agents' | 'agentsAdd' | 'switch' | 'switchCode';

export interface Initial {
  openId?: number | null;
  tab?: DetailTab;
  lane?: Lane;
  dialog?: DialogName;
  /** The agent the switch dialog is about. */
  switchAgent?: string;
  /** The agent a finished change went to (with `done`). */
  to?: string;
  /** Opens the card's assignee menu (the sheet on a phone). */
  menu?: boolean;
  /** Shows the result of an operation that was just confirmed. */
  done?: 'reassign';
}

export interface AssignmentScreenProps {
  variant?: 'desktop' | 'phone';
  initial?: Initial;
}

export type Notice = {
  kind: 'reassign';
  id: number;
  from: string;
  to: string;
  g0: number;
  g1: number;
};

export const handOver = (cards: MockCard[], id: number, to: string): MockCard[] =>
  cards.map((c) =>
    c.id === id
      ? { ...c, agentId: to, generation: c.generation + 1, running: false, fresh: true }
      : { ...c, fresh: false },
  );

/** The cards and the notice a story starts with: the board, or the board after a confirmed operation. */
export function initialState(
  ja: boolean,
  initial: Initial,
): { cards: MockCard[]; notice: Notice | null } {
  let cards = buildFixture(ja).cards;
  let notice: Notice | null = null;
  if (initial.done === 'reassign') {
    const card = cards.find((c) => c.id === 28);
    cards = handOver(cards, 28, initial.to ?? 'A-3');
    notice = {
      kind: 'reassign',
      id: 28,
      from: 'A-2',
      to: initial.to ?? 'A-3',
      g0: card?.generation ?? 2,
      g1: (card?.generation ?? 2) + 1,
    };
  }
  return { cards, notice };
}
