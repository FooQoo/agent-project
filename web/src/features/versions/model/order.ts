const collator = new Intl.Collator('en', { numeric: true, sensitivity: 'base' });

/**
 * The versions newest first. The compare is by natural order (`v1.10.0` after `v1.9.0`), so it does
 * not depend on the order the server lists them in; names that compare equal keep a stable order.
 */
export function sortVersions(versions: readonly string[]): string[] {
  return [...versions].sort((a, b) => collator.compare(b, a) || (a < b ? 1 : a > b ? -1 : 0));
}
