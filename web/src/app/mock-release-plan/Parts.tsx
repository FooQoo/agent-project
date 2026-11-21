import { FlagIcon, TagIcon, XIcon } from 'lucide-react';
import { type ReactNode, useId } from 'react';
import '../../shared/styles/modal.css';
import { Dialog, DialogContent, DialogTitle } from '@/shared/ui/dialog';
import { Drawer } from '@/shared/ui/drawer';
import { RadioGroupItem } from '@/shared/ui/radio-group';
import { useMockText } from './text';
import './release-plan-mock.css';

/** The planned version as a card shows it: a tag, with a mark when it is the current version. */
export function VersionTag({
  planned,
  current,
}: {
  planned: string | null;
  current: string | null;
}) {
  const m = useMockText();
  const kind = planned === null ? 'none' : planned === current ? 'current' : 'other';
  return (
    <span className="rp-tag" data-kind={kind}>
      <TagIcon aria-hidden="true" />
      <span>{planned ?? m.plannedNone}</span>
      {kind === 'current' && <span className="rp-tag__mark">{m.plannedCurrentMark}</span>}
    </span>
  );
}

/** Where a mock screen opens: a modal on the desktop board, a bottom sheet on a phone. */
export function Surface({
  kind,
  title,
  identity,
  onClose,
  footer,
  className = '',
  children,
}: {
  kind: 'dialog' | 'sheet';
  title: string;
  identity?: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  const m = useMockText();
  if (kind === 'sheet') {
    return (
      <Drawer
        open
        onOpenChange={(open) => !open && onClose()}
        identity={identity ?? ''}
        title={title}
        closeLabel={m.close}
        className={`rp-sheet ${className}`.trim()}
        initialFocusSelector=".rp-surface__body"
      >
        <div className="rp-surface__body" tabIndex={-1}>
          {children}
        </div>
        {footer && <div className="rp-surface__footer">{footer}</div>}
      </Drawer>
    );
  }
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        className={`modal rp-modal ${className}`.trim()}
        showCloseButton={false}
        aria-describedby={undefined}
        onOpenAutoFocus={(event) => {
          event.preventDefault();
          if (event.currentTarget instanceof HTMLElement)
            event.currentTarget.focus({ preventScroll: true });
        }}
      >
        <div className="modal__header">
          <DialogTitle className="modal__title">{title}</DialogTitle>
          <button type="button" className="modal__close" aria-label={m.close} onClick={onClose}>
            <XIcon aria-hidden="true" />
          </button>
        </div>
        <div className="rp-surface__body">{children}</div>
        {footer && <div className="rp-surface__footer">{footer}</div>}
      </DialogContent>
    </Dialog>
  );
}

/** Whose change an operation is: the whole project, or one task. Shown on both operations. */
export function ScopeBadge({ scope, children }: { scope: 'project' | 'task'; children: string }) {
  return (
    <span className="rp-scope" data-scope={scope}>
      {scope === 'project' ? <FlagIcon aria-hidden="true" /> : <TagIcon aria-hidden="true" />}
      {children}
    </span>
  );
}

export function OptionText({ label, hint }: { label: string; hint?: string }) {
  return (
    <span className="rp-opt">
      <span className="rp-opt__label">{label}</span>
      {hint && <span className="rp-opt__hint">{hint}</span>}
    </span>
  );
}

/** One radio choice as a whole-row target; the label names the radio through `htmlFor`. */
export function OptionRow({
  value,
  checked,
  disabled,
  label,
  hint,
}: {
  value: string;
  checked: boolean;
  disabled?: boolean;
  label: string;
  hint?: string;
}) {
  const id = useId();
  return (
    <label htmlFor={id} className="rp-option" data-checked={checked} data-disabled={disabled}>
      <RadioGroupItem id={id} value={value} disabled={disabled} />
      <OptionText label={label} hint={hint} />
    </label>
  );
}

/** One radio choice drawn as a pill, for a short list of versions. */
export function PillRow({
  value,
  checked,
  label,
  mark,
}: {
  value: string;
  checked: boolean;
  label: string;
  mark?: string;
}) {
  const id = useId();
  return (
    <label htmlFor={id} className="rp-pill" data-checked={checked}>
      <RadioGroupItem id={id} value={value} className="rp-pill__radio" />
      <span>{label}</span>
      {mark && <span className="rp-tag__mark">{mark}</span>}
    </label>
  );
}
