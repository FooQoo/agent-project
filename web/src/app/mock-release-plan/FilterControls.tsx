import { ChevronDownIcon, TagIcon } from 'lucide-react';
import '../../shared/styles/modal.css';
import { Button } from '@/shared/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/shared/ui/dropdown-menu';
import { RadioGroup } from '@/shared/ui/radio-group';
import { VERSIONS, type VersionFilter } from './data';
import { type MockText, useMockText } from './text';
import { OptionRow, OptionText, Surface } from './Parts';
import './release-plan-mock.css';

export function filterText(m: MockText, filter: VersionFilter, current: string | null): string {
  switch (filter.kind) {
    case 'all':
      return m.filterAll;
    case 'current':
      return `${m.filterCurrent} (${current ?? m.filterCurrentUnset})`;
    case 'version':
      return filter.version;
    case 'none':
      return m.filterNone;
  }
}

export function filterKey(filter: VersionFilter): string {
  return filter.kind === 'version' ? `v:${filter.version}` : filter.kind;
}

export function filterFromKey(key: string): VersionFilter {
  if (key.startsWith('v:')) return { kind: 'version', version: key.slice(2) };
  if (key === 'current') return { kind: 'current' };
  if (key === 'none') return { kind: 'none' };
  return { kind: 'all' };
}

/** The filter button of the bar; on the desktop it opens a menu, on a phone it opens a sheet. */
export function FilterButton({
  m,
  filter,
  current,
  variant,
  onFilter,
  onOpenSheet,
}: {
  m: MockText;
  filter: VersionFilter;
  current: string | null;
  variant: 'desktop' | 'phone';
  onFilter: (next: VersionFilter) => void;
  onOpenSheet: () => void;
}) {
  const label = (
    <>
      <TagIcon aria-hidden="true" />
      <span className="rp-filter__name">{m.filterButton}</span>
      <strong className="rp-filter__value">{filterText(m, filter, current)}</strong>
      <ChevronDownIcon aria-hidden="true" />
    </>
  );
  if (variant === 'phone') {
    return (
      <Button
        type="button"
        variant="outline"
        size="l"
        className="rp-filter"
        aria-haspopup="dialog"
        aria-label={`${m.filterLabel}: ${filterText(m, filter, current)}`}
        onClick={onOpenSheet}
      >
        {label}
      </Button>
    );
  }
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="outline"
          className="rp-filter"
          aria-label={`${m.filterLabel}: ${filterText(m, filter, current)}`}
        >
          {label}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="rp-menu">
        <DropdownMenuRadioGroup
          value={filterKey(filter)}
          onValueChange={(key) => onFilter(filterFromKey(key))}
        >
          <DropdownMenuRadioItem value="all">
            <OptionText label={m.filterAll} />
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="current">
            <OptionText
              label={`${m.filterCurrent} (${current ?? m.filterCurrentUnset})`}
              hint={current === null ? m.filterCurrentUnsetHint : m.filterCurrentHint}
            />
          </DropdownMenuRadioItem>
          <DropdownMenuLabel>{m.filterGroupVersions}</DropdownMenuLabel>
          {VERSIONS.map((v) => (
            <DropdownMenuRadioItem key={v} value={`v:${v}`}>
              <OptionText label={v} />
            </DropdownMenuRadioItem>
          ))}
          <DropdownMenuLabel>{m.plannedNone}</DropdownMenuLabel>
          <DropdownMenuRadioItem value="none">
            <OptionText label={m.filterNone} hint={m.filterNoneHint} />
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The phone's filter as a bottom sheet: the same choices, in large rows. */
export function FilterSheet({
  filter,
  current,
  onPick,
  onClose,
}: {
  filter: VersionFilter;
  current: string | null;
  onPick: (next: VersionFilter) => void;
  onClose: () => void;
}) {
  const m = useMockText();
  const row = (key: string, label: string, hint?: string) => (
    <OptionRow
      key={key}
      value={key}
      checked={filterKey(filter) === key}
      label={label}
      hint={hint}
    />
  );
  return (
    <Surface kind="sheet" title={m.filterLabel} onClose={onClose}>
      <RadioGroup
        value={filterKey(filter)}
        onValueChange={(key) => {
          onPick(filterFromKey(key));
          onClose();
        }}
        className="rp-options"
        aria-label={m.filterLabel}
      >
        {row('all', m.filterAll)}
        {row(
          'current',
          `${m.filterCurrent} (${current ?? m.filterCurrentUnset})`,
          current === null ? m.filterCurrentUnsetHint : m.filterCurrentHint,
        )}
        <p className="rp-eyebrow">{m.filterGroupVersions}</p>
        {VERSIONS.map((v) => row(`v:${v}`, v))}
        <p className="rp-eyebrow">{m.plannedNone}</p>
        {row('none', m.filterNone, m.filterNoneHint)}
      </RadioGroup>
    </Surface>
  );
}
