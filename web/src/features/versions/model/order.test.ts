import { describe, expect, it } from 'vitest';
import { sortVersions } from './order';

describe('the order of versions', () => {
  it('puts the newest first, comparing the numbers as numbers', () => {
    expect(sortVersions(['v1.8.0', 'v1.10.0', 'v1.9.0', 'v2.0.0'])).toEqual([
      'v2.0.0',
      'v1.10.0',
      'v1.9.0',
      'v1.8.0',
    ]);
  });

  it('does not depend on the order it is given in, and leaves its input alone', () => {
    const given = ['v1.9.0', 'v1.8.0', 'v1.10.0'];
    expect(sortVersions(given)).toEqual(sortVersions([...given].reverse()));
    expect(given).toEqual(['v1.9.0', 'v1.8.0', 'v1.10.0']);
  });

  it('orders names with no number too', () => {
    expect(sortVersions(['alpha', 'beta', 'v1.0.0'])).toEqual(['v1.0.0', 'beta', 'alpha']);
  });
});
