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

export interface ReleaseVersionMenuProps {
  versions: readonly string[];
  /** The project's release version; `null` when none is set. */
  release: string | null;
  /** Makes a listed version the release version, or clears the release version with `null`. */
  onSelect: (version: string | null) => Promise<unknown>;
  /** Lists a new version. It does not become the release version: only choosing it from the list does that. */
  onAdd: (name: string) => Promise<unknown>;
  disabled?: boolean;
}

/**
 * The project's release version, in the header beside the project's name. It opens a list of the
 * project's versions, newest first: choosing one makes it the release version. The top row is a text field:
 * typing a version and confirming only lists it; the release version changes when one is chosen
 * from the list. "No release version" is the state a project starts in.
 */
export function ReleaseVersionMenu({
  versions,
  release,
  onSelect,
  onAdd,
  disabled = false,
}: ReleaseVersionMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const listed = sortVersions(
    release !== null && !versions.includes(release) ? [...versions, release] : versions,
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
  const value = release ?? t('versions.header.unset');
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
          data-set={release !== null}
        >
          {release === null ? <FlagOffIcon aria-hidden="true" /> : <FlagIcon aria-hidden="true" />}
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
          value={release ?? NONE}
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
              checked={release === version}
              label={version}
              mark={release === version ? t('versions.header.markRelease') : undefined}
            />
          ))}
          <OptionRow value={NONE} checked={release === null} label={t('versions.header.none')} />
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
