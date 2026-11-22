import { CheckIcon, ChevronDownIcon, FlagIcon, FlagOffIcon } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { Input } from '@/shared/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/ui/popover';
import { RadioGroup } from '@/shared/ui/radio-group';
import { sortVersions } from '../model/order';
import { OptionRow } from './Options';
import './versions.css';

const NONE = '__none__';

export interface CurrentVersionMenuProps {
  versions: readonly string[];
  /** The project's current version; `null` when none is set. */
  current: string | null;
  /** Makes a listed version current, or clears the current version with `null`. */
  onSelect: (version: string | null) => Promise<unknown>;
  /** Lists a new version. It does not become current: only choosing it from the list does that. */
  onAdd: (name: string) => Promise<unknown>;
  disabled?: boolean;
}

/**
 * The project's current version, in the header beside the project's name. It opens a list of the
 * project's versions, newest first: choosing one makes it current. The top row is a text field:
 * typing a version and confirming only lists it; the current version changes when one is chosen
 * from the list. "No current version" is the state a project starts in.
 */
export function CurrentVersionMenu({
  versions,
  current,
  onSelect,
  onAdd,
  disabled = false,
}: CurrentVersionMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const listed = sortVersions(
    current !== null && !versions.includes(current) ? [...versions, current] : versions,
  );

  const run = async (change: () => Promise<unknown>, done: () => void) => {
    if (busy) return;
    setBusy(true);
    setFailed(false);
    try {
      await change();
      done();
    } catch {
      setFailed(true);
    } finally {
      setBusy(false);
    }
  };

  const name = draft.trim();
  const value = current ?? t('versions.header.unset');
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) setFailed(false);
      }}
    >
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          className="ver-head"
          disabled={disabled}
          aria-label={t('versions.header.buttonValue', { version: value })}
          data-set={current !== null}
        >
          {current === null ? <FlagOffIcon aria-hidden="true" /> : <FlagIcon aria-hidden="true" />}
          <span className="ver-head__label">{t('versions.header.button')}</span>
          <strong className="ver-head__value">{value}</strong>
          <ChevronDownIcon aria-hidden="true" />
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="ver-head-pop">
        <form
          className="ver-head-add"
          onSubmit={(event) => {
            event.preventDefault();
            // Listing a version keeps the list open: the person then chooses it, or another one.
            if (name !== '')
              void run(
                () => onAdd(name),
                () => setDraft(''),
              );
          }}
        >
          <Input
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder={t('versions.header.addPlaceholder')}
            aria-label={t('versions.header.addLabel')}
            maxLength={64}
            autoComplete="off"
          />
          <Button
            type="submit"
            iconOnly
            disabled={busy || name === ''}
            aria-label={t('versions.header.addConfirm')}
            title={t('versions.header.addConfirm')}
          >
            <CheckIcon aria-hidden="true" />
          </Button>
        </form>
        <RadioGroup
          value={current ?? NONE}
          onValueChange={(next) =>
            void run(
              () => onSelect(next === NONE ? null : next),
              () => setOpen(false),
            )
          }
          className="ver-options ver-head-list"
          aria-label={t('versions.header.listLabel')}
        >
          {listed.length === 0 && <p className="ver-note">{t('versions.header.empty')}</p>}
          {listed.map((version) => (
            <OptionRow
              key={version}
              value={version}
              checked={current === version}
              label={version}
              mark={current === version ? t('versions.header.markCurrent') : undefined}
            />
          ))}
          <OptionRow value={NONE} checked={current === null} label={t('versions.header.none')} />
        </RadioGroup>
        {failed && (
          <p className="ver-error" role="alert">
            {t('versions.header.failed')}
          </p>
        )}
      </PopoverContent>
    </Popover>
  );
}
