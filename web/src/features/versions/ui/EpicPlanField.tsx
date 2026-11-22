import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Checkbox } from '@/shared/ui/checkbox';
import { RadioGroup } from '@/shared/ui/radio-group';
import type { PendingSave } from '../../../shared/ui/editor/planField';
import type { EpicVersionResult, EpicVersionScope, VersionChoice } from '../api/versions';
import {
  type BulkMember,
  type BulkRow,
  type BulkTarget,
  bulkRows,
  checkedIds,
  choiceOf,
  targetName,
} from '../model/bulk';
import { sortVersions } from '../model/order';
import { OptionRow, PillRow } from './Options';
import './versions.css';

export interface EpicPlanFieldProps {
  /** The epic's tasks the board lists. */
  members: readonly BulkMember[];
  current: string | null;
  versions: readonly string[];
  disabled?: boolean;
  /** The change that would be made, with nothing written (`dryRun: true`). */
  preview: (
    choice: VersionChoice,
    scope: EpicVersionScope,
    taskIds: number[],
  ) => Promise<EpicVersionResult>;
  /** The same body with `dryRun: false`. */
  apply: (
    choice: VersionChoice,
    scope: EpicVersionScope,
    taskIds: number[],
  ) => Promise<EpicVersionResult>;
  /** Reports the pending change: the editor runs it when it saves. `null` when there is none. */
  onPending: (pending: PendingSave | null) => void;
}

// Controls and versions have disjoint values, so a version may be named anything.
const KEEP = 'c:keep';
const NONE = 'c:none';
const CURRENT = 'c:current';

function keyOf(target: BulkTarget | null): string {
  if (target === null) return KEEP;
  if (target.kind === 'unset') return NONE;
  if (target.kind === 'current') return CURRENT;
  return `v:${target.version}`;
}

function targetOf(key: string): BulkTarget | null {
  if (key === KEEP) return null;
  if (key === NONE) return { kind: 'unset' };
  if (key === CURRENT) return { kind: 'current' };
  return { kind: 'named', version: key.slice(2) };
}

/**
 * Plans every task of an epic for one version, inside the epic's editor. Choosing a version asks the
 * server for a dry run, so each task's change (or why it is skipped) is shown before anything is
 * saved. The person can uncheck rows; the editor applies the change with the same body and
 * `dryRun: false` when it saves. Done tasks are always skipped.
 */
export function EpicPlanField({
  members,
  current,
  versions,
  disabled = false,
  preview,
  apply,
  onPending,
}: EpicPlanFieldProps) {
  const { t } = useTranslation();
  const [target, setTarget] = useState<BulkTarget | null>(null);
  const [scope, setScope] = useState<EpicVersionScope>('unplanned-only');
  const [rows, setRows] = useState<BulkRow[] | null>(null);
  const [unchecked, setUnchecked] = useState<Set<number>>(new Set());
  const [failed, setFailed] = useState(false);
  const serial = useRef(0);
  const previewRef = useRef(preview);
  previewRef.current = preview;
  const applyRef = useRef(apply);
  applyRef.current = apply;
  const memberKey = members
    .map((member) => `${member.id}:${member.plannedVersion ?? ''}`)
    .join('|');

  // biome-ignore lint/correctness/useExhaustiveDependencies: `memberKey` stands for the members; `current` for the target's name
  useEffect(() => {
    const mine = ++serial.current;
    setRows(null);
    setFailed(false);
    if (target === null) return;
    previewRef
      .current(
        choiceOf(target),
        scope,
        members.map((member) => member.id),
      )
      .then((result) => {
        if (serial.current === mine) setRows(bulkRows(members, result));
      })
      .catch(() => {
        if (serial.current === mine) setFailed(true);
      });
  }, [target, scope, memberKey, current]);

  const ids = rows === null ? [] : checkedIds(rows, unchecked);
  const idsKey = ids.join(',');
  const resolved = rows !== null;
  // The change the editor saves. It holds the checked rows as they are now, so a later update of
  // the board cannot change what Save applies. A chosen target whose preview is not ready is an
  // unfinished step: Save fails it instead of closing without the change. "No change" and a
  // resolved preview with every row unchecked have nothing to save.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `idsKey` stands for the checked rows
  useEffect(() => {
    if (target === null) onPending(null);
    else if (!resolved)
      onPending(async () => {
        throw new Error('The preview of the planned versions is not ready.');
      });
    else if (ids.length === 0) onPending(null);
    else {
      const checked = [...ids];
      onPending(async () => void (await applyRef.current(choiceOf(target), scope, checked)));
    }
  }, [target, scope, idsKey, resolved, onPending]);

  const name = target === null ? '' : (targetName(target, current) ?? t('versions.tag.none'));
  const unplannedCount = members.filter((member) => member.plannedVersion === null).length;
  const named = sortVersions(versions.filter((version) => version !== current));
  return (
    <section className="ver-epic-plan" aria-labelledby="ver-epic-plan-title">
      <h3 className="sr-only" id="ver-epic-plan-title">
        {t('versions.bulk.title')}
      </h3>
      <p className="ver-note">{t('versions.bulk.note')}</p>
      <RadioGroup
        value={keyOf(target)}
        onValueChange={(key) => {
          // A different bulk operation starts again with every task checked.
          setUnchecked(new Set());
          setTarget(targetOf(key));
        }}
        disabled={disabled}
        className="ver-pills"
        aria-label={t('versions.bulk.target')}
      >
        <PillRow value={KEEP} checked={target === null} label={t('versions.bulk.noChange')} />
        {current !== null && (
          <PillRow
            value={CURRENT}
            checked={target?.kind === 'current'}
            label={t('versions.detail.current', { version: current })}
          />
        )}
        {named.map((version) => (
          <PillRow
            key={version}
            value={`v:${version}`}
            checked={target?.kind === 'named' && target.version === version}
            label={version}
          />
        ))}
        <PillRow value={NONE} checked={target?.kind === 'unset'} label={t('versions.tag.none')} />
      </RadioGroup>
      {target !== null && (
        <>
          <RadioGroup
            value={scope}
            onValueChange={(next) => {
              setUnchecked(new Set());
              setScope(next === 'all' ? 'all' : 'unplanned-only');
            }}
            disabled={disabled}
            className="ver-options"
            aria-label={t('versions.bulk.scope')}
          >
            <OptionRow
              value="unplanned-only"
              checked={scope === 'unplanned-only'}
              label={`${t('versions.bulk.scopeUnplanned')} (${unplannedCount})`}
            />
            <OptionRow
              value="all"
              checked={scope === 'all'}
              label={`${t('versions.bulk.scopeAll')} (${members.length})`}
            />
          </RadioGroup>
          <p className="ver-summary" data-empty={ids.length === 0} role="status" aria-live="polite">
            {rows === null && !failed
              ? t('versions.bulk.checking')
              : ids.length === 0
                ? t('versions.bulk.summaryNone')
                : t('versions.bulk.summary', {
                    changed: ids.length,
                    total: members.length,
                    to: name,
                  })}
          </p>
          {failed && (
            <p className="ver-error" role="alert">
              {t('versions.bulk.checkFailed')}
            </p>
          )}
          {members.length === 0 ? (
            <p className="ver-note">{t('versions.bulk.noTasks')}</p>
          ) : (
            <>
              <p className="ver-note">{t('versions.bulk.listHint')}</p>
              <ul className="ver-targets" aria-busy={rows === null}>
                {members.map((member) => {
                  const row = rows?.find((item) => item.id === member.id);
                  return (
                    <PlanRow
                      key={member.id}
                      member={member}
                      row={row}
                      name={name}
                      checked={row?.state === 'change' && !unchecked.has(member.id)}
                      disabled={disabled}
                      onToggle={(next) =>
                        setUnchecked((prev) => {
                          const copy = new Set(prev);
                          if (next) copy.delete(member.id);
                          else copy.add(member.id);
                          return copy;
                        })
                      }
                    />
                  );
                })}
              </ul>
            </>
          )}
        </>
      )}
    </section>
  );
}

function PlanRow({
  member,
  row,
  name,
  checked,
  disabled,
  onToggle,
}: {
  member: BulkMember;
  row: BulkRow | undefined;
  name: string;
  checked: boolean;
  disabled: boolean;
  onToggle: (checked: boolean) => void;
}) {
  const { t } = useTranslation();
  const state =
    row === undefined ? 'pending' : !checked && row.state === 'change' ? 'off' : row.state;
  const id = `ver-t-${member.id}`;
  return (
    <li className="ver-target" data-state={state}>
      <label htmlFor={id} className="ver-target__label">
        <Checkbox
          id={id}
          checked={checked}
          disabled={disabled || row?.state !== 'change'}
          onCheckedChange={(next) => onToggle(next === true)}
        />
        <span className="ver-target__title">
          <span className="ver-target__id">#{member.id}</span> {member.title}
        </span>
      </label>
      <span className="ver-target__change">
        <span className="ver-from">{member.plannedVersion ?? t('versions.tag.none')}</span>
        {row?.state === 'change' && checked && (
          <>
            <span aria-hidden="true">→</span>
            <strong>{name}</strong>
          </>
        )}
        {row?.state === 'same' && <span className="ver-same">{t('versions.bulk.unchanged')}</span>}
        {row?.state === 'skipped' && (
          <span className="ver-same">{t(`versions.bulk.skipped.${row.reason}`)}</span>
        )}
      </span>
    </li>
  );
}
