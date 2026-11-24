import { ImagePlusIcon } from 'lucide-react';
import { useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/shared/ui/button';
import { Input } from '@/shared/ui/input';
import { ComposerThumbs } from '../../../shared/ui/editor/ComposerThumbs';
import { attachmentNoticeText } from '../../../shared/ui/editor/AttachmentPanel';
import { useAttachmentState } from '../../../shared/ui/editor/useAttachmentState';
import type { PlanDelivery } from './deliverPlan';

/**
 * The form in a plan's row for registering the real result: the summary, pictures, and a link. All
 * three are optional, because the summary reads as the plan's content when left empty. When the
 * request fails the input stays, the reason shows, and the button reads "Try again".
 */
export function PlanRegisterForm({
  planId,
  placeholder,
  onSubmit,
  onCancel,
}: {
  planId: string;
  /** Shown in the summary field: the content of the plan, which the summary defaults to. */
  placeholder: string;
  onSubmit: (delivery: PlanDelivery) => Promise<void>;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const id = useId();
  const [summary, setSummary] = useState('');
  const [link, setLink] = useState('');
  const [failed, setFailed] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const picker = useRef<HTMLInputElement>(null);
  const pictures = useAttachmentState({ resolveStoredUrl: () => '' });
  const submit = async () => {
    if (busy) return;
    setBusy(true);
    setFailed(null);
    try {
      await onSubmit({ summary, link, attachments: pictures.attachments });
    } catch (error) {
      setFailed(error instanceof Error && error.message !== '' ? error.message : '');
      setBusy(false);
    }
  };
  return (
    <form
      className="plan-register"
      aria-label={t('resultPlans.registerFor', { id: planId })}
      onSubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <label className="plan-register__field" htmlFor={`${id}-summary`}>
        <span className="plan-register__caption">{t('resultPlans.summary')}</span>
      </label>
      <textarea
        id={`${id}-summary`}
        className="plan-register__summary"
        rows={3}
        value={summary}
        disabled={busy}
        placeholder={placeholder}
        onChange={(event) => setSummary(event.target.value)}
      />
      <label className="plan-register__field" htmlFor={`${id}-link`}>
        <span className="plan-register__caption">{t('resultPlans.link')}</span>
      </label>
      <Input
        id={`${id}-link`}
        type="text"
        value={link}
        disabled={busy}
        placeholder={t('resultPlans.linkPlaceholder')}
        onChange={(event) => setLink(event.target.value)}
      />
      <div className="plan-register__images">
        <input
          ref={picker}
          type="file"
          accept="image/png,image/jpeg,image/webp,image/gif"
          multiple
          hidden
          aria-label={t('resultPlans.addImages')}
          onChange={(event) => {
            pictures.addFiles(Array.from(event.target.files ?? []));
            event.target.value = '';
          }}
        />
        <Button
          type="button"
          variant="outline"
          size="s"
          disabled={busy}
          onClick={() => picker.current?.click()}
        >
          <ImagePlusIcon aria-hidden="true" focusable="false" />
          <span>{t('resultPlans.addImages')}</span>
        </Button>
        {pictures.error && (
          <p className="plan-register__error" role="alert">
            {attachmentNoticeText(pictures.error, t)}
          </p>
        )}
      </div>
      {pictures.attachments.length > 0 && (
        <ComposerThumbs
          attachments={pictures.attachments}
          label={t('resultPlans.images')}
          removeLabel={(name) => t('resultPlans.removeImage', { name })}
          disabled={busy}
          onRemove={pictures.remove}
        />
      )}
      {failed !== null && (
        <p className="plan-register__error" role="alert">
          {t('resultPlans.registerFailed')}
          {failed !== '' && ` ${failed}`}
        </p>
      )}
      <div className="plan-register__actions">
        <Button type="button" variant="outline" size="s" disabled={busy} onClick={onCancel}>
          {t('resultPlans.cancel')}
        </Button>
        <Button type="submit" size="s" disabled={busy}>
          {busy
            ? t('resultPlans.registering')
            : failed !== null
              ? t('resultPlans.retry')
              : t('resultPlans.register')}
        </Button>
      </div>
    </form>
  );
}
