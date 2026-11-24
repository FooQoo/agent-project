import type { DetailTab } from '../../features/tasks/detail/DetailTabs';
import { type Lane, type MockCard, buildFixture, bulkTargets } from './data';

export type DialogName = 'agents' | 'agentsAdd' | 'switch' | 'switchCode' | 'reassign' | 'bulk';

export interface Initial {
  openId?: number | null;
  tab?: DetailTab;
  lane?: Lane;
  dialog?: DialogName;
  /** The agent the switch dialog is about. */
  switchAgent?: string;
  /** The choice already made in the handover or the bulk dialog. */
  to?: string;
  epicId?: string;
  /** Shows the result of an operation that was just confirmed. */
  done?: 'reassign' | 'bulk';
}

export interface AssignmentScreenProps {
  variant?: 'desktop' | 'phone';
  initial?: Initial;
}

export type Notice =
  | { kind: 'reassign'; id: number; from: string; to: string; g0: number; g1: number }
  | { kind: 'bulk'; ids: number[]; to: string; kept: number };

export const handOver = (cards: MockCard[], id: number, to: string): MockCard[] =>
  cards.map((c) =>
    c.id === id
      ? { ...c, agentId: to, generation: c.generation + 1, running: false, fresh: true }
      : { ...c, fresh: false },
  );

/** Moves every open AI card of the epic to `to`; the others stay. */
export const bulkAssign = (cards: MockCard[], epicId: string, to: string): MockCard[] => {
  const moved = new Set(bulkTargets(cards, epicId, to).map((c) => c.id));
  return cards.map((c) =>
    moved.has(c.id)
      ? { ...c, agentId: to, generation: c.generation + 1, fresh: true }
      : { ...c, fresh: false },
  );
};

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
  if (initial.done === 'bulk') {
    const epicId = initial.epicId ?? 'LOGIN-1';
    const ids = bulkTargets(cards, epicId, initial.to ?? 'A-2').map((c) => c.id);
    cards = bulkAssign(cards, epicId, initial.to ?? 'A-2');
    notice = {
      kind: 'bulk',
      ids,
      to: initial.to ?? 'A-2',
      kept: cards.filter((c) => c.epicId === epicId).length - ids.length,
    };
  }
  return { cards, notice };
}
