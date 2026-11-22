import type { PlanProject } from '../../../features/versions/ui/PlanContext';
import type { Summary } from '../../../shared/data/api';

/**
 * The versions the phone's cards show their tag from. The phone has no way to set a version: it
 * only reads the planned version of a card. Without `enabled` (the demo's sample data, a story) no
 * card shows a tag.
 */
export function planProject(enabled: boolean, summary: Summary | null): PlanProject | null {
  if (!enabled || summary === null) return null;
  return { versions: summary.project.versions, current: summary.project.currentVersion };
}
