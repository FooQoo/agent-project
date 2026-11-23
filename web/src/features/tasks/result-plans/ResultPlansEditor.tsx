import { PlusIcon, XIcon } from 'lucide-react';
import { useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { Checkbox } from '@/shared/ui/checkbox';
import { Input } from '@/shared/ui/input';
import { nextTempId, planProblem, type ResultPlanItem } from './model/resultPlans';
import './result-plans.css';

/**
 * The planned results of a card, edited in its dialog: for each one, what will be delivered, in
 * which state it should be delivered, and whether it is the final result. A plan that was
 * delivered cannot be removed, and only one plan is final: choosing another moves the mark.
 */
export function ResultPlansEditor({
  items,
  onChange,
  disabled = false,
}: {
  items: readonly ResultPlanItem[];
  onChange: (items: ResultPlanItem[]) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const base = useId();
  const problem = planProblem(items);
  const [focusId, setFocusId] = useState<string | null>(null);
  const inputs = useRef(new Map<string, HTMLInputElement>());
  useEffect(() => {
    if (focusId === null) return;
    inputs.current.get(focusId)?.focus();
    setFocusId(null);
  }, [focusId]);
  const update = (index: number, change: Partial<ResultPlanItem>) =>
    onChange(
      items.map((item, i) => {
        if (i === index) return { ...item, ...change };
        // At most one plan is final.
        return change.final === true ? { ...item, final: false } : item;
      }),
    );
  const add = () => {
    const id = nextTempId(items);
    onChange([...items, { id, content: '', condition: '', final: false, delivered: null }]);
    setFocusId(id);
  };
  return (
    <div className="plans-edit">
      <ol className="plans-edit__list">
        {items.map((item, index) => {
          const label = item.id.startsWith('new-') ? t('resultPlans.newPlan') : item.id;
          const empty = problem?.kind === 'empty' && problem.index === index;
          return (
            <li key={item.id} className="plans-edit__row" data-id={item.id}>
              <div className="plans-edit__head">
                <span className="plans-edit__id">{label}</span>
                {item.delivered !== null && (
                  <span className="plans-edit__delivered">
                    {t('resultPlans.deliveredAs', { version: item.delivered })}
                  </span>
                )}
                <span className="plans-edit__spacer" />
                <span className="plans-edit__final">
                  <Checkbox
                    id={`${base}-${item.id}-final`}
                    checked={item.final}
                    disabled={disabled}
                    onCheckedChange={(checked) => update(index, { final: checked === true })}
                  />
                  <label htmlFor={`${base}-${item.id}-final`}>{t('resultPlans.final')}</label>
                </span>
                <Button
                  type="button"
                  variant="ghost"
                  size="s"
                  iconOnly
                  aria-label={t('resultPlans.remove', { id: label })}
                  disabled={disabled || item.delivered !== null}
                  title={item.delivered !== null ? t('resultPlans.removeDelivered') : undefined}
                  onClick={() => onChange(items.filter((_, i) => i !== index))}
                >
                  <XIcon aria-hidden="true" focusable="false" />
                </Button>
              </div>
              <div className="plans-edit__field">
                <span className="plans-edit__caption">{t('resultPlans.content')}</span>
                <Input
                  ref={(element) => {
                    if (element) inputs.current.set(item.id, element);
                    else inputs.current.delete(item.id);
                  }}
                  type="text"
                  value={item.content}
                  aria-label={t('resultPlans.content')}
                  disabled={disabled}
                  placeholder={t('resultPlans.contentPlaceholder')}
                  aria-invalid={empty && item.content.trim() === '' ? 'true' : undefined}
                  onChange={(event) => update(index, { content: event.target.value })}
                />
              </div>
              <div className="plans-edit__field">
                <span className="plans-edit__caption">{t('resultPlans.condition')}</span>
                <Input
                  type="text"
                  value={item.condition}
                  aria-label={t('resultPlans.condition')}
                  disabled={disabled}
                  placeholder={t('resultPlans.conditionPlaceholder')}
                  aria-invalid={empty && item.condition.trim() === '' ? 'true' : undefined}
                  onChange={(event) => update(index, { condition: event.target.value })}
                />
              </div>
              {empty && (
                <span className="plans-edit__error" role="alert">
                  {t('resultPlans.emptyField')}
                </span>
              )}
            </li>
          );
        })}
      </ol>
      {problem?.kind === 'finalMany' && (
        <p className="plans-edit__error" role="alert">
          {t('resultPlans.finalMany')}
        </p>
      )}
      <Button
        type="button"
        variant="outline"
        size="s"
        className="plans-edit__add"
        disabled={disabled}
        onClick={add}
      >
        <PlusIcon aria-hidden="true" focusable="false" />
        <span>{t('resultPlans.add')}</span>
      </Button>
      <p className="plans-edit__note">{t('resultPlans.note')}</p>
    </div>
  );
}
