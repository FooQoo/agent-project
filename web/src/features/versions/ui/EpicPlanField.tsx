import { ChevronDownIcon } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/ui/popover';
import { RadioGroup } from '@/shared/ui/radio-group';
import type { PendingSave } from '../../../shared/ui/editor/planField';
import type { EpicVersionResult, VersionChoice } from '../api/versions';
import { sortVersions } from '../model/order';
import { OptionRow } from './Options';
import './versions.css';

export interface EpicPlanFieldProps {
  current: string | null;
  versions: readonly string[];
  disabled?: boolean;
  /** Plans every task of the epic that is not done for the choice (the server skips the rest). */
  apply: (choice: VersionChoice) => Promise<EpicVersionResult>;
  /** Reports the pending change: the editor runs it when it saves. `null` for "No change". */
  onPending: (pending: PendingSave | null) => void;
}

// Controls and versions have disjoint values, so a version may be named anything.
const KEEP = 'c:keep';
const NONE = 'c:none';
const CURRENT = 'c:current';

function choiceOf(key: string): VersionChoice | null {
  if (key === KEEP) return null;
  if (key === NONE) return { choice: 'unset' };
  if (key === CURRENT) return { choice: 'current' };
  return { choice: 'named', version: key.slice(2) };
}

/**
 * The planned version of an epic's tasks, inside the epic's editor: no change (the default), none,
 * the current version, or a version of the list. Saving the editor plans every task of the epic that
 * is not done for that choice. The choice is taken when it is made, so a later update of the board
 * cannot change what Save applies; a single task is changed afterwards in its own editor.
 */
export function EpicPlanField({
  current,
  versions,
  disabled = false,
  apply,
  onPending,
}: EpicPlanFieldProps) {
  const { t } = useTranslation();
  const [key, setKey] = useState(KEEP);
  const applyRef = useRef(apply);
  applyRef.current = apply;
  useEffect(() => {
    const choice = choiceOf(key);
    onPending(choice === null ? null : async () => void (await applyRef.current(choice)));
    return () => onPending(null);
  }, [key, onPending]);
  const [open, setOpen] = useState(false);
  const named = sortVersions(versions.filter((version) => version !== current));
  const labels = new Map<string, string>([
    [KEEP, t('versions.bulk.noChange')],
    [
      CURRENT,
      current === null
        ? t('versions.bulk.currentUnset')
        : t('versions.bulk.current', { version: current }),
    ],
    [NONE, t('versions.detail.none')],
  ]);
  const shown = labels.get(key) ?? key.slice(2);
  const titleId = 'ver-epic-plan-title';
  const choose = (next: string) => {
    setKey(next);
    setOpen(false);
  };
  return (
    <section className="ver-epic-plan" aria-labelledby={titleId}>
      <h3 className="sr-only" id={titleId}>
        {t('versions.bulk.title')}
      </h3>
      <p className="ver-note">{t('versions.bulk.note')}</p>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            className="ver-select"
            disabled={disabled}
            aria-label={`${t('versions.bulk.target')}: ${shown}`}
            data-changed={key !== KEEP}
          >
            <span className="ver-select__value">{shown}</span>
            <ChevronDownIcon aria-hidden="true" />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="start" className="ver-select-pop">
          <RadioGroup
            value={key}
            onValueChange={choose}
            className="ver-options ver-head-list"
            aria-label={t('versions.bulk.target')}
          >
            <OptionRow value={KEEP} checked={key === KEEP} label={labels.get(KEEP) ?? ''} />
            <OptionRow
              value={CURRENT}
              checked={key === CURRENT}
              disabled={current === null}
              label={labels.get(CURRENT) ?? ''}
              hint={current === null ? t('versions.detail.currentUnsetHint') : undefined}
            />
            {named.map((version) => (
              <OptionRow
                key={version}
                value={`v:${version}`}
                checked={key === `v:${version}`}
                label={version}
              />
            ))}
            <OptionRow value={NONE} checked={key === NONE} label={labels.get(NONE) ?? ''} />
          </RadioGroup>
        </PopoverContent>
      </Popover>
    </section>
  );
}
