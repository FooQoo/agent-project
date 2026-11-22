import { createContext, useContext } from 'react';
import { VersionTag, showsVersionTag } from './VersionTag';

/**
 * The project's versions for the parts of the phone screen that sit deep in its tree (the card
 * and the detail drawer). A screen without planning (the demo's sample data, a story) provides none,
 * so those parts draw nothing.
 */
export interface PlanProject {
  versions: readonly string[];
  current: string | null;
}

export const PlanContext = createContext<PlanProject | null>(null);

export function usePlanProject(): PlanProject | null {
  return useContext(PlanContext);
}

/** The card's planned-version tag, when the screen plans versions. */
export function PlanTag({ planned }: { planned: string | null }) {
  const project = usePlanProject();
  if (project === null || !showsVersionTag(planned, project)) return null;
  return <VersionTag planned={planned} current={project.current} />;
}
