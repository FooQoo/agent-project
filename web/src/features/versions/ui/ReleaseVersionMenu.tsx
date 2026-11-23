import { CheckIcon, ChevronDownIcon, FlagIcon, FlagOffIcon } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { Input } from '@/shared/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/ui/popover';
import { RadioGroup } from '@/shared/ui/radio-group';
import { Surface } from '../../../shared/ui/surface/Surface';
import { sortVersions } from '../model/order';
import { OptionRow } from './Options';
import './versions.css';

const ALL = '__all__';

export interface ReleaseVersionMenuProps {
  versions: readonly string[];
  /** The project's release version; `null` when none is set. */
  release: string | null;
  /** The version the board is filtered by; `null` shows every version ("All"). */
  filter: string | null;
  /** Filters the board by a listed version, or shows all versions with `null`. View-only: nothing is saved. */
  onFilter: (version: string | null) => void;
  /**
   * Makes a version the project's release version. Without it the menu offers no such button: the
   * phone leaves the release version to the PC.
   */
  onMakeRelease?: (version: string) => Promise<unknown>;
  /** Lists a new version. It changes neither the filter nor the release version. */
  onAdd: (name: string) => Promise<unknown>;
  disabled?: boolean;
  /** `popover` opens beside the button (the PC header); `sheet` opens a bottom sheet (the phone). */
  kind?: 'popover' | 'sheet';
}

/**
 * The version filter of the board, in the header beside the project's name. It opens a list of the
 * project's versions, newest first, under "All": choosing one filters the Todo, Doing and Done lanes
 * to it, and "All" clears the filter. Choosing only changes this screen. The release version is marked
 * with a star and changes only with the button at the bottom. The top row is a text field: typing a
 * version and confirming only lists it.
 */
export function ReleaseVersionMenu({
  versions,
  release,
  filter,
  onFilter,
  onMakeRelease,
  onAdd,
  disabled = false,
  kind = 'popover',
}: ReleaseVersionMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const extra = [release, filter].filter(
    (version): version is string => version !== null && !versions.includes(version),
  );
  const listed = sortVersions([...versions, ...extra]);

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
  const value = filter ?? t('versions.header.all');
  const trigger = (
    <Button
      type="button"
      variant="outline"
      className="ver-head"
      disabled={disabled}
      aria-label={t('versions.header.buttonValue', { version: value })}
      data-set={filter !== null}
      onClick={kind === 'sheet' ? () => setOpen(true) : undefined}
    >
      {filter === null ? <FlagOffIcon aria-hidden="true" /> : <FlagIcon aria-hidden="true" />}
      <strong className="ver-head__value">{value}</strong>
      <ChevronDownIcon aria-hidden="true" />
    </Button>
  );
  const makeRelease =
    onMakeRelease && filter !== null && filter !== release ? (
      <Button
        type="button"
        variant="outline"
        disabled={busy}
        onClick={() =>
          void run(
            () => onMakeRelease(filter),
            () => setOpen(false),
          )
        }
      >
        {t('versions.header.makeRelease', { version: filter })}
      </Button>
    ) : null;
  const body = (
    <>
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
        value={filter ?? ALL}
        onValueChange={(next) => {
          onFilter(next === ALL ? null : next);
          setOpen(false);
        }}
        className="ver-options ver-head-list"
        aria-label={t('versions.header.listLabel')}
      >
        <OptionRow value={ALL} checked={filter === null} label={t('versions.header.all')} />
        {listed.map((version) => (
          <OptionRow
            key={version}
            value={version}
            checked={filter === version}
            label={version}
            mark={release === version ? '\u2605' : undefined}
            markLabel={release === version ? t('versions.header.markRelease') : undefined}
          />
        ))}
        {listed.length === 0 && <p className="ver-note">{t('versions.header.empty')}</p>}
      </RadioGroup>
      {kind === 'popover' && makeRelease}
      {failed && (
        <p className="ver-error" role="alert">
          {t('versions.header.failed')}
        </p>
      )}
    </>
  );
  if (kind === 'sheet')
    return (
      <>
        {trigger}
        {open && (
          <Surface
            kind="sheet"
            title={t('versions.header.sheetTitle')}
            closeLabel={t('dialog.close')}
            onClose={() => {
              setOpen(false);
              setFailed(false);
            }}
            className="ver-head-sheet"
            footer={makeRelease}
          >
            {body}
          </Surface>
        )}
      </>
    );
  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) setFailed(false);
      }}
    >
      <PopoverTrigger asChild>{trigger}</PopoverTrigger>
      <PopoverContent align="start" className="ver-head-pop">
        {body}
      </PopoverContent>
    </Popover>
  );
}
