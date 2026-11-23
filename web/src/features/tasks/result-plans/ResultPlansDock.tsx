import { ChevronDownIcon, PackageCheckIcon } from 'lucide-react';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ResultPlansEditor } from './ResultPlansEditor';
import type { ResultPlanItem } from './model/resultPlans';
import './plans-dock.css';
import './result-plans.css';

/**
 * The planned results of a card, held at the bottom of its edit dialog beside the conditions. It
 * starts folded to a line with the count, and shows its editor when opened.
 */
export function ResultPlansDock({
  items,
  onChange,
  disabled = false,
  defaultOpen = false,
}: {
  items: readonly ResultPlanItem[];
  onChange: (items: ResultPlanItem[]) => void;
  disabled?: boolean;
  defaultOpen?: boolean;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(defaultOpen);
  const bodyId = useId();
  const delivered = items.filter((item) => item.delivered !== null).length;
  return (
    <section className="plans-dock" data-open={open ? 'true' : undefined}>
      <button
        type="button"
        className="plans-dock__head"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => setOpen((current) => !current)}
      >
        <PackageCheckIcon aria-hidden="true" focusable="false" className="plans-dock__icon" />
        <span className="plans-dock__title">{t('resultPlans.title')}</span>
        <span className="plans-dock__count">
          {t('resultPlans.progress', { delivered, total: items.length })}
        </span>
        <ChevronDownIcon aria-hidden="true" focusable="false" className="plans-dock__chevron" />
      </button>
      <div id={bodyId} className="plans-dock__body" hidden={!open}>
        <ResultPlansEditor items={items} onChange={onChange} disabled={disabled} />
      </div>
    </section>
  );
}
