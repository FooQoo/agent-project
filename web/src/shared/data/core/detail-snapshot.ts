// What a screen is shown of one section of a Detail, from what its query says.
import { summaryState } from './summary-reader';
import type { SectionDataMap, SectionSnapshot } from './types';

/** A Detail section as the query holds it: a whole version and the version it has. */
export interface DetailEntry<S extends keyof SectionDataMap = keyof SectionDataMap> {
  data: SectionDataMap[S];
  version: string;
}

export const ABSENT_SECTION: SectionSnapshot<never> = Object.freeze({
  state: 'absent',
  data: null,
  partial: null,
  version: null,
  error: null,
}) as SectionSnapshot<never>;

export function sectionSnapshot<S extends keyof SectionDataMap>(query: {
  entry: DetailEntry<S> | undefined;
  fetching: boolean;
  failed: boolean;
  stale: boolean;
  error: Error | null;
  partial: SectionDataMap[S] | null;
}): SectionSnapshot<S> {
  return {
    state: summaryState({
      hasData: query.entry !== undefined,
      fetching: query.fetching,
      failed: query.failed,
      stale: query.stale,
    }),
    data: query.entry?.data ?? null,
    partial: query.fetching ? query.partial : null,
    version: query.entry?.version ?? null,
    error: query.error,
  };
}
