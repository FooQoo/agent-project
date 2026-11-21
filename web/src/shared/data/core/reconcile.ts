// After a new Summary: the comparison of the cached Details with the versions it names.
import type { ProjectId } from '../../api/contracts';
import type { DetailSection, DetailVersions, Summary } from '../api/contracts';
import { detailId, type FlightHost, keyOf } from './refresh-types';

/**
 * Compares the cached Details with the versions the new Summary names: the ones that match are
 * whole again, the ones that differ are fetched, the ones it no longer lists are dropped. This
 * is what makes the cache right even when a notification was missed.
 */
export function reconcile(
  host: FlightHost,
  projectId: ProjectId,
  summary: Summary,
  immediate: boolean,
): void {
  for (const key of host.store.cachedKeys(projectId)) {
    if (host.hasFlight(detailId(key))) continue;
    const entry =
      key.kind === 'task'
        ? summary.tasks.find((t) => t.id === key.id)
        : summary.epics.find((e) => e.id === key.id);
    if (!entry) {
      host.cancelDetail(key);
      host.store.remove(key);
      continue;
    }
    const versions = entry.detailVersions as DetailVersions;
    const differing: DetailSection[] = [];
    for (const section of host.store.cachedSections(key)) {
      const expected = versions[section];
      if (expected === undefined) continue;
      host.unverified.delete(`${keyOf(key)}|${section}`);
      if (host.store.getSection(key, section).version === expected) {
        host.store.markFresh(key, section);
      } else {
        host.store.markStale(key, section);
        differing.push(section);
      }
    }
    if (differing.length > 0) {
      host.enqueue({
        projectId,
        key,
        sections: differing,
        immediate,
        waiters: [],
      });
    }
  }
}
