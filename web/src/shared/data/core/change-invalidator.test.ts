// @vitest-environment node
import { beforeEach, describe, expect, it } from 'vitest';
import type { Change, ChangedEvent } from '../api/contracts';
import { ChangeInvalidator } from './change-invalidator';
import { epic, P, task } from './test-support';
import type { ResourceKey } from './types';

let invalidator: ChangeInvalidator;
/** The sections the cache holds a query for, as `kind:id:section`. */
let held: Set<string>;

const event = (changes: Change[], seq = 1, projectId = P, instanceId = 'i1'): ChangedEvent => ({
  instanceId,
  seq,
  projectId,
  changes,
});
const change = (resource: Change['resource'], id: number | string, ...facets: Change['facets']) =>
  ({ resource, id, facets }) as Change;
const nameOf = (key: ResourceKey, section: string) => `${key.kind}:${key.id}:${section}`;

beforeEach(() => {
  held = new Set([
    nameOf(task(1), 'document'),
    nameOf(task(1), 'history'),
    nameOf(epic('E'), 'document'),
  ]);
  invalidator = new ChangeInvalidator({
    isActiveProject: (id) => id === P,
    hasDetail: (key, section) => held.has(nameOf(key, section)),
  });
});

describe('when the Summary must be read again', () => {
  it('asks for it when a section of a cached task changed, since it names the new version', () => {
    expect(invalidator.handle(event([change('task', 1, 'history')])).summary).toBe(true);
    expect(invalidator.handle(event([change('task', 1, 'document')], 2)).summary).toBe(true);
  });

  it('asks for it when the document of a cached epic changed', () => {
    expect(invalidator.handle(event([change('epic', 'E', 'document')])).summary).toBe(true);
  });

  it('does not ask for a section of a resource that is not cached', () => {
    expect(invalidator.handle(event([change('task', 99, 'document', 'history')])).summary).toBe(
      false,
    );
    expect(invalidator.handle(event([change('epic', 'Z', 'document')], 2)).summary).toBe(false);
  });

  it('asks for it on the summary facet, whatever is cached', () => {
    expect(invalidator.handle(event([change('task', 99, 'summary')])).summary).toBe(true);
  });

  it('asks for it for a project change', () => {
    expect(invalidator.handle(event([change('project', P, 'summary')])).summary).toBe(true);
  });

  it('asks for it when a resource is deleted', () => {
    expect(invalidator.handle(event([change('task', 1, 'deleted')])).summary).toBe(true);
  });

  it('ignores another project', () => {
    expect(invalidator.handle(event([change('task', 1, 'document')], 1, 'other')).summary).toBe(
      false,
    );
  });
});

describe('when the cache cannot be trusted', () => {
  it('accepts the first event and the next sequence number', () => {
    expect(invalidator.handle(event([], 7)).reset).toBeNull();
    expect(invalidator.handle(event([], 8)).reset).toBeNull();
  });

  it('resets on a missing sequence number', () => {
    invalidator.handle(event([], 7));
    const result = invalidator.handle(event([], 9));
    expect(result.reset).toBe('seq-gap');
    expect(result.summary).toBe(true);
  });

  it('resets when the sequence goes back or repeats', () => {
    invalidator.handle(event([], 7));
    expect(invalidator.handle(event([], 7)).reset).toBe('seq-gap');
  });

  it('resets when the server instance changes, and counts from the new one afterwards', () => {
    invalidator.handle(event([], 7));
    expect(invalidator.handle(event([], 1, P, 'i2')).reset).toBe('instance-changed');
    expect(invalidator.handle(event([], 2, P, 'i2')).reset).toBeNull();
  });

  it('checks the sequence on events of other projects too', () => {
    invalidator.handle(event([], 7, 'other'));
    expect(invalidator.handle(event([], 9, 'other')).reset).toBe('seq-gap');
  });

  it('resets on a resource or a facet it does not know', () => {
    const unknownResource = { resource: 'sprint', id: 1, facets: ['summary'] } as unknown as Change;
    expect(invalidator.handle(event([unknownResource], 1)).reset).toBe('unknown-event');
    const unknownFacet = { resource: 'task', id: 1, facets: ['comments'] } as unknown as Change;
    expect(invalidator.handle(event([unknownFacet], 2)).reset).toBe('unknown-event');
  });

  it('knows request for a task only', () => {
    const asked = invalidator.handle(event([change('task', 1, 'summary', 'request')], 1));
    expect(asked).toMatchObject({ reset: null, summary: true });
    const onEpic = change('epic', 'E', 'summary', 'request');
    expect(invalidator.handle(event([onEpic], 2)).reset).toBe('unknown-event');
  });

  it('forgets the sequence so a reconnect does not look like a gap', () => {
    invalidator.handle(event([], 7));
    invalidator.forgetSequence();
    expect(invalidator.handle(event([], 20)).reset).toBeNull();
  });

  it('reset() asks for a new Summary for a reason the connection found', () => {
    expect(invalidator.reset('reconnected')).toEqual({ reset: 'reconnected', summary: true });
  });
});

describe('observe and apply apart', () => {
  it('checks the sequence on receipt and decides nothing until apply', () => {
    expect(invalidator.observe(event([change('task', 1, 'document')], 1))).toBeNull();
    expect(invalidator.apply(event([change('task', 1, 'document')], 1)).summary).toBe(true);
  });
});
