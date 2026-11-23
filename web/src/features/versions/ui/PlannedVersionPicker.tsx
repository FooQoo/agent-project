import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RadioGroup } from '@/shared/ui/radio-group';
import type { PendingSave } from '../../../shared/ui/editor/planField';
import { PlannedVersionChangedError, type VersionChoice } from '../api/versions';
import { sortVersions } from '../model/order';
import { OptionRow } from './Options';
import './versions.css';

// Controls and versions have disjoint values, so a version may be named anything.
const NONE = 'c:none';
const RELEASE = 'c:release';

export interface PlannedVersionPickerProps {
  taskId: number;
  /** The planned version the page shows; `null` when none. */
  planned: string | null;
  release: string | null;
  versions: readonly string[];
  disabled?: boolean;
  /**
   * Saves the choice over the value shown (`expected`). Rejects with `PlannedVersionChangedError`
   * when the task has another planned version now.
   */
  onSave: (choice: VersionChoice, expected: string | null) => Promise<unknown>;
  /** Reads the board again; called when the task changed under the picker. */
  onRefetch?: () => void;
  /** Reports the pending change: the editor runs it when it saves. `null` when nothing differs. */
  onPending: (pending: PendingSave | null) => void;
}

/** What the saved state looks like among the rows. */
function savedKey(planned: string | null, release: string | null): string {
  if (planned === null) return NONE;
  return planned === release ? RELEASE : `v:${planned}`;
}

export function choiceOf(key: string): VersionChoice {
  if (key === NONE) return { choice: 'unset' };
  if (key === RELEASE) return { choice: 'release' };
  return { choice: 'named', version: key.slice(2) };
}

type Notice = { kind: 'failed' } | { kind: 'conflict'; version: string | null };

/**
 * The planned version of one task, inside the task's editor: the release version (saving its name),
 * a version of the list, or none. The choice is saved with the editor, over the value this shows.
 * When the task changed meanwhile the save is refused, the newer value is reported and the board is
 * read again; the choice is never written silently, and the person decides again.
 */
export function PlannedVersionPicker({
  taskId,
  planned,
  release,
  versions,
  disabled = false,
  onSave,
  onRefetch,
  onPending,
}: PlannedVersionPickerProps) {
  const { t } = useTranslation();
  const [pick, setPick] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const saved = savedKey(planned, release);
  const value = pick ?? saved;
  const others = sortVersions(versions.filter((version) => version !== release));
  const listed = planned !== null && planned !== release && !others.includes(planned);
  const rows = listed ? [...others, planned] : others;

  // The pending change saves over the value the person saw when choosing. When the task changed
  // meanwhile the save is refused and the newer value is reported; the change is then armed again
  // over that newer value, so only a second, deliberate save replaces it.
  const report = useCallback(
    (key: string | null, expected: string | null) =>
      onPending(
        key === null
          ? null
          : async () => {
              try {
                await onSave(choiceOf(key), expected);
                setPick(null);
              } catch (cause) {
                if (cause instanceof PlannedVersionChangedError) {
                  setNotice({ kind: 'conflict', version: cause.current });
                  onRefetch?.();
                  report(key, cause.current);
                } else setNotice({ kind: 'failed' });
                throw cause;
              }
            },
      ),
    [onPending, onSave, onRefetch],
  );

  const choose = (next: string) => {
    const key = next === saved ? null : next;
    setPick(key);
    setNotice(null);
    report(key, planned);
  };

  const titleId = `ver-picker-${taskId}`;
  return (
    <section className="ver-picker" aria-labelledby={titleId}>
      <h3 className="ver-picker__title" id={titleId}>
        {t('versions.detail.title')}
      </h3>
      <p className="ver-note">
        {release === null
          ? t('versions.detail.noteUnset')
          : t('versions.detail.note', { version: release })}
      </p>
      <RadioGroup
        value={value}
        onValueChange={choose}
        disabled={disabled}
        className="ver-options"
        aria-labelledby={titleId}
      >
        <OptionRow
          value={RELEASE}
          checked={value === RELEASE}
          disabled={release === null}
          label={
            release === null
              ? t('versions.detail.releaseUnset')
              : t('versions.detail.release', { version: release })
          }
          hint={
            release === null
              ? t('versions.detail.releaseUnsetHint')
              : t('versions.detail.releaseHint')
          }
        />
        {rows.map((version) => (
          <OptionRow
            key={version}
            value={`v:${version}`}
            checked={value === `v:${version}`}
            label={version}
          />
        ))}
        <OptionRow value={NONE} checked={value === NONE} label={t('versions.detail.none')} />
      </RadioGroup>
      {notice?.kind === 'conflict' && (
        <p className="ver-error" role="alert">
          {notice.version === null
            ? t('versions.detail.conflictNone')
            : t('versions.detail.conflict', { version: notice.version })}
        </p>
      )}
      {notice?.kind === 'failed' && (
        <p className="ver-error" role="alert">
          {t('versions.detail.failed')}
        </p>
      )}
    </section>
  );
}
