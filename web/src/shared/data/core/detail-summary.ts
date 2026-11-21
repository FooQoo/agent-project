// What a Summary says about one card or epic, and how a cached query maps back to its resource.
import type { Query } from '@tanstack/react-query';
import type { ProjectId } from '../../api/contracts';
import type { DetailSection, DetailVersions, Summary } from '../api/contracts';
import type { ResourceKey } from './types';

/** The resource and section a detail query holds (the query key says both). */
export function keyOfQuery(query: Query): { key: ResourceKey; section: DetailSection } {
  const [kind, projectId, id, section] = query.queryKey as [
    ResourceKey['kind'],
    ProjectId,
    number | string,
    DetailSection,
  ];
  return { key: { kind, projectId, id } as ResourceKey, section };
}

/** The Summary's line for a resource, if it lists it. */
export function entryOf(summary: Summary, key: ResourceKey) {
  return key.kind === 'task'
    ? summary.tasks.find((t) => t.id === key.id)
    : summary.epics.find((e) => e.id === key.id);
}

/** The versions a (trusted) Summary names for the sections of a resource, if it lists it. */
export function namedVersions(
  summary: Summary | null,
  key: ResourceKey,
  sections: readonly DetailSection[],
): DetailVersions | undefined {
  const named = (summary && entryOf(summary, key))?.detailVersions as DetailVersions | undefined;
  if (!named) return undefined;
  const versions: DetailVersions = {};
  for (const section of sections)
    if (named[section] !== undefined) versions[section] = named[section];
  return versions;
}

/** The sections whose held version is not the one the Summary names (none the Summary does not name). */
export function outdatedSections(
  summary: Summary | null,
  key: ResourceKey,
  sections: readonly DetailSection[],
  held: (section: DetailSection) => string | null,
): DetailSection[] {
  const named = (summary && namedVersions(summary, key, sections)) ?? {};
  return sections.filter(
    (section) => named[section] !== undefined && named[section] !== held(section),
  );
}
