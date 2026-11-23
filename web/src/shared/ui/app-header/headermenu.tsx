import type { ComponentType, RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { ArchiveIcon, BookOpenIcon, LayersIcon, MenuIcon, SettingsIcon } from 'lucide-react';
import { Button } from '@/shared/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/shared/ui/dropdown-menu';

export interface HeaderMenuProps {
  /** Opens the project's docs; the item is left out without it. */
  onDocs?: () => void;
  /** Opens the epic screen; left out without it. */
  onEpics?: () => void;
  /** Opens the archive; left out without it. */
  onArchive?: () => void;
  /** Opens the settings dialog; left out without it. */
  onSettings?: () => void;
  /** A card form or confirmation is open, so the board's destinations are disabled. */
  switchDisabled?: boolean;
  /** The menu button, which takes the focus back when the settings dialog closes. */
  triggerRef: RefObject<HTMLButtonElement | null>;
}

function Entry({
  icon: Icon,
  label,
  className,
  disabled,
  onSelect,
}: {
  icon: ComponentType<{ 'aria-hidden'?: boolean }>;
  label: string;
  className: string;
  disabled?: boolean;
  onSelect: () => void;
}) {
  return (
    <DropdownMenuItem className={className} disabled={disabled} onSelect={onSelect}>
      <Icon aria-hidden={true} />
      <span>{label}</span>
    </DropdownMenuItem>
  );
}

/**
 * The one menu at the right end of the PC header: Docs, Epics, Archive and Settings. Ask AI, the
 * project and the version stay in the header itself, so the bar never has to squeeze their names.
 */
export function HeaderMenu({
  onDocs,
  onEpics,
  onArchive,
  onSettings,
  switchDisabled = false,
  triggerRef,
}: HeaderMenuProps) {
  const { t } = useTranslation();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          ref={triggerRef}
          type="button"
          variant="outline"
          iconOnly
          className="header__menu"
          aria-label={t('headerMenu.label')}
        >
          <MenuIcon aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="header-menu" aria-label={t('headerMenu.label')}>
        {onDocs && (
          <Entry
            icon={BookOpenIcon}
            label={t('docs.title')}
            className="header__docs"
            disabled={switchDisabled}
            onSelect={onDocs}
          />
        )}
        {onEpics && (
          <Entry
            icon={LayersIcon}
            label={t('epics.title')}
            className="header__epics"
            disabled={switchDisabled}
            onSelect={onEpics}
          />
        )}
        {onArchive && (
          <Entry
            icon={ArchiveIcon}
            label={t('archive.title')}
            className="header__archive"
            disabled={switchDisabled}
            onSelect={onArchive}
          />
        )}
        {onSettings && (
          <Entry
            icon={SettingsIcon}
            label={t('settings.title')}
            className="header__settings"
            onSelect={onSettings}
          />
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
