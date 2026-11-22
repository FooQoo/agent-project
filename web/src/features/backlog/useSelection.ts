import { useCallback, useEffect, useRef, useState } from 'react';

export interface LaneSelection {
  selecting: boolean;
  /** The picked cards that are still in the lane, in the order of the lane. */
  picked: number[];
  enter: () => void;
  leave: () => void;
  toggle: (id: number) => void;
}

/** The Backlog's selection under its own name. */
export type BacklogSelection = LaneSelection;

/**
 * The selection mode of a lane: a mode that is entered and left, and a set of picked cards. A card
 * that left the lane (sent or archived from another screen, or by an agent's change) is no longer
 * picked. With `preselect`, entering picks every card of the lane (the Done lane's archive); without
 * it, entering picks nothing (the Backlog's send).
 */
export function useLaneSelection(
  laneIds: readonly number[],
  options: { preselect?: boolean } = {},
): LaneSelection {
  const preselect = options.preselect === true;
  const [selecting, setSelecting] = useState(false);
  const [raw, setRaw] = useState<number[]>([]);
  const latest = useRef(laneIds);
  latest.current = laneIds;
  const picked = laneIds.filter((id) => raw.includes(id));
  // Once every picked card has left the lane (they were sent or archived), the mode ends.
  const doneAll = raw.length > 0 && picked.length === 0;
  useEffect(() => {
    if (doneAll) {
      setSelecting(false);
      setRaw([]);
    }
  }, [doneAll]);
  const enter = useCallback(() => {
    setSelecting(true);
    setRaw(preselect ? [...latest.current] : []);
  }, [preselect]);
  const leave = useCallback(() => {
    setSelecting(false);
    setRaw([]);
  }, []);
  const toggle = useCallback(
    (id: number) =>
      setRaw((now) => (now.includes(id) ? now.filter((i) => i !== id) : [...now, id])),
    [],
  );
  return { selecting: selecting && laneIds.length > 0, picked, enter, leave, toggle };
}

/** The Backlog lane's selection: it starts with nothing picked. */
export const useBacklogSelection = (laneIds: readonly number[]) => useLaneSelection(laneIds);
