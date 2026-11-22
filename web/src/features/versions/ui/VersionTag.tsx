import { TagIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

/**
 * The planned version of a card, in a slot of its own: a tag, marked when it is the project's
 * current version, and a dashed "unplanned" tag when the card has none.
 */
export function VersionTag({
  planned,
  current,
  className = '',
}: {
  planned: string | null;
  current: string | null;
  className?: string;
}) {
  const { t } = useTranslation();
  const kind = planned === null ? 'none' : planned === current ? 'current' : 'other';
  return (
    <span className={`ver-tag ${className}`.trim()} data-kind={kind}>
      <TagIcon aria-hidden="true" />
      <span>{planned ?? t('versions.tag.none')}</span>
      {kind === 'current' && <span className="ver-tag__mark">{t('versions.tag.current')}</span>}
    </span>
  );
}

/**
 * Whether a card of this project shows the tag. A project that has never used versions shows none,
 * so its cards stay as they were; a planned card always shows its version.
 */
export function showsVersionTag(
  planned: string | null,
  project: { versions: readonly string[]; current: string | null },
): boolean {
  return planned !== null || project.versions.length > 0 || project.current !== null;
}
