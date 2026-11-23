import { CheckCircle2Icon, CircleDashedIcon } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import type { ResultPlanView } from '../../../shared/api/contracts';
import type { PlanDelivery } from './deliverPlan';
import { PlanRegisterForm } from './PlanRegisterForm';
import { deliveredCount } from './model/resultPlans';
import './result-plans.css';

/**
 * The plans of a task at the top of its Results tab. Each row says what is to be delivered and when,
 * whether it was delivered, and either opens the result that delivered it or, for a person who may
 * record results, registers the real result in place.
 */
export function ResultPlansSection({
  plans,
  canRegister,
  onRegister,
  onOpenResult,
}: {
  plans: readonly ResultPlanView[];
  /** The person may register a result: not a draft or an archived task. */
  canRegister: boolean;
  onRegister?: (plan: ResultPlanView, delivery: PlanDelivery) => Promise<void>;
  onOpenResult: (version: number) => void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState<string | null>(null);
  if (plans.length === 0) return null;
  return (
    <section className="plans" aria-label={t('resultPlans.title')}>
      <header className="plans__head">
        <h3 className="plans__title">{t('resultPlans.title')}</h3>
        <span className="plans__count">
          {t('resultPlans.progress', { delivered: deliveredCount(plans), total: plans.length })}
        </span>
      </header>
      <ul className="plans__list">
        {plans.map((plan) => {
          const delivered = plan.deliveredResultVersion !== null;
          return (
            <li
              key={plan.id}
              className="plans__row"
              data-plan={plan.id}
              data-state={delivered ? 'delivered' : 'pending'}
            >
              <div className="plans__meta">
                <span className="plans__id">{plan.id}</span>
                <span className="plans__kind" data-final={plan.final ? 'true' : undefined}>
                  {plan.final ? t('resultPlans.final') : t('resultPlans.intermediate')}
                </span>
                <span className="plans__state" data-state={delivered ? 'delivered' : 'pending'}>
                  {delivered ? (
                    <CheckCircle2Icon aria-hidden="true" focusable="false" />
                  ) : (
                    <CircleDashedIcon aria-hidden="true" focusable="false" />
                  )}
                  {delivered ? t('resultPlans.delivered') : t('resultPlans.notDelivered')}
                </span>
              </div>
              <p className="plans__content">{plan.content}</p>
              <p className="plans__condition">
                <span className="plans__condition-label">{t('resultPlans.when')}</span>
                {plan.condition}
              </p>
              {delivered && plan.deliveredResultVersion !== null && (
                <Button
                  type="button"
                  variant="outline"
                  size="s"
                  className="plans__action"
                  onClick={() => onOpenResult(plan.deliveredResultVersion as number)}
                >
                  {t('resultPlans.openResult', { version: plan.deliveredResultVersion })}
                </Button>
              )}
              {!delivered && canRegister && onRegister && open !== plan.id && (
                <Button
                  type="button"
                  size="s"
                  className="plans__action"
                  onClick={() => setOpen(plan.id)}
                >
                  {t('resultPlans.registerResult')}
                </Button>
              )}
              {!delivered && canRegister && onRegister && open === plan.id && (
                <PlanRegisterForm
                  planId={plan.id}
                  placeholder={plan.content}
                  onCancel={() => setOpen(null)}
                  onSubmit={async (delivery) => {
                    await onRegister(plan, delivery);
                    setOpen(null);
                  }}
                />
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
