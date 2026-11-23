import { Suspense, useEffect, useRef, useState, type ReactNode, type RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowLeftIcon, SettingsIcon, SparklesIcon } from 'lucide-react';
import { Button, type ButtonSize } from '@/shared/ui/button';
import { IconLabel } from '@/shared/ui/icon-label';
import { HeaderMenu } from './headermenu';
import type { Project, ProjectId } from '../../api/contracts';

export type ViewMode = 'board' | 'archive' | 'epics';

/** What the header hands to the settings dialog it hosts. */
export interface SettingsSlotState {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The settings button, which takes the focus back when the dialog closes. */
  restoreFocusRef: RefObject<HTMLButtonElement | null>;
}

/** The settings dialog, composed by the page. Its code may load after the header is shown. */
export interface HeaderSettings {
  render: (state: SettingsSlotState) => ReactNode;
  /** Fetches the dialog's code once the header is shown (see `preloadable`). */
  preload?: () => Promise<unknown>;
}

export interface HeaderProps {
  /** The project list; `null` until it is known, which hides the switcher. */
  projects?: Project[] | null;
  activeProjectId?: ProjectId | null;
  /** A card form or confirmation is open, so the project cannot change. */
  switchDisabled?: boolean;
  /** Add card is offered only once the board of the project has loaded. */
  canAdd?: boolean;
  /** Opens "Ask AI". */
  onAdd?: () => void;
  onArchive?: () => void;
  onEpics?: () => void;
  /** Opens the project's docs, from the menu. Offered with Epics and Archive, on the board only. */
  onDocs?: () => void;
  viewMode?: ViewMode;
  onBack?: () => void;
  /** The project switcher, shown after the slash when the project list is known. */
  projectSwitcher?: ReactNode;
  /** The project's release-version menu, shown right of the project's name on the board. */
  versionSlot?: ReactNode;
  /** The settings dialog; without it the settings button opens nothing. */
  settings?: HeaderSettings;
  /** The logo the header shows; the board's own logo file by default. */
  logoSrc?: string;
  /** The size of the settings button; the monitor uses a larger one for a thumb. */
  settingsSize?: ButtonSize;
  /** Shows the settings button. The passcode screen leaves it out: nothing can be saved there. */
  withSettings?: boolean;
  /**
   * Gathers Docs, Epics, Archive and Settings into one menu at the right end, as the PC board does.
   * Without it the header keeps a plain settings button, as the phone screens do.
   */
  menu?: boolean;
}

export function Header({
  projects = null,
  activeProjectId = null,
  switchDisabled = false,
  canAdd = false,
  onAdd = () => {},
  onArchive = () => {},
  onEpics = () => {},
  onDocs,
  viewMode = 'board',
  onBack = () => {},
  projectSwitcher = null,
  versionSlot = null,
  settings,
  logoSrc = `${import.meta.env.BASE_URL}logo-white.png`,
  settingsSize = 'm',
  withSettings = true,
  menu = false,
}: HeaderProps) {
  const { t } = useTranslation();
  const [settingsOpen, setSettingsOpen] = useState(false);
  useEffect(() => {
    // A failed fetch is tried again when the dialog is first rendered.
    settings?.preload?.().catch(() => {});
  }, [settings]);
  const settingsButtonRef = useRef<HTMLButtonElement>(null);
  const menuButtonRef = useRef<HTMLButtonElement>(null);

  return (
    <header className="header">
      <h1 className="header__app">
        <span className="header__logo">
          <img src={logoSrc} alt="" width="24" height="24" />
        </span>
        <span>Agent Project</span>
      </h1>
      {projects !== null && (
        <>
          <span className="header__slash" aria-hidden="true">
            /
          </span>
          {viewMode === 'epics' ? (
            // The epic screen shows the project's name as text, as its design does; switch projects from the board.
            <span className="header__board-name">
              {projects.find((project) => project.id === activeProjectId)?.name ?? ''}
            </span>
          ) : (
            <div className="header__switcher">{projectSwitcher}</div>
          )}
          {versionSlot && viewMode === 'board' && (
            <div className="header__version">{versionSlot}</div>
          )}
        </>
      )}
      <span className="header__spacer" aria-hidden="true" />
      {viewMode === 'board' && canAdd && (
        <Button type="button" className="header__add" disabled={switchDisabled} onClick={onAdd}>
          <IconLabel icon={SparklesIcon}>{t('askAi.button')}</IconLabel>
        </Button>
      )}
      {viewMode === 'archive' || viewMode === 'epics' ? (
        <Button
          type="button"
          variant="outline"
          className="header__back"
          disabled={switchDisabled}
          onClick={onBack}
        >
          {viewMode === 'epics' ? (
            <IconLabel icon={ArrowLeftIcon}>{t('epics.back')}</IconLabel>
          ) : (
            t('archive.back')
          )}
        </Button>
      ) : null}
      {menu ? (
        <HeaderMenu
          triggerRef={menuButtonRef}
          switchDisabled={switchDisabled}
          onDocs={viewMode === 'board' && canAdd ? onDocs : undefined}
          onEpics={viewMode === 'board' && canAdd ? onEpics : undefined}
          onArchive={viewMode === 'board' && canAdd ? onArchive : undefined}
          onSettings={withSettings ? () => setSettingsOpen(true) : undefined}
        />
      ) : (
        withSettings && (
          <Button
            ref={settingsButtonRef}
            type="button"
            variant="outline"
            size={settingsSize}
            iconOnly
            className="header__settings"
            aria-label={t('settings.title')}
            aria-haspopup="dialog"
            onClick={() => setSettingsOpen(true)}
          >
            <SettingsIcon aria-hidden="true" />
          </Button>
        )
      )}
      {withSettings && (
        <Suspense fallback={null}>
          {settings?.render({
            open: settingsOpen,
            onOpenChange: setSettingsOpen,
            restoreFocusRef: menu ? menuButtonRef : settingsButtonRef,
          })}
        </Suspense>
      )}
    </header>
  );
}
