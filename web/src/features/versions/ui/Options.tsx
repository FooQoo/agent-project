import { useId } from 'react';
import { RadioGroupItem } from '@/shared/ui/radio-group';
import './versions.css';

export function OptionText({ label, hint }: { label: string; hint?: string }) {
  return (
    <span className="ver-opt">
      <span className="ver-opt__label">{label}</span>
      {hint && <span className="ver-opt__hint">{hint}</span>}
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
  mark,
}: {
  value: string;
  checked: boolean;
  disabled?: boolean;
  label: string;
  hint?: string;
  /** A small mark after the label, such as "Current". */
  mark?: string;
}) {
  const id = useId();
  return (
    <label htmlFor={id} className="ver-option" data-checked={checked} data-disabled={disabled}>
      <RadioGroupItem id={id} value={value} disabled={disabled} />
      <OptionText label={label} hint={hint} />
      {mark && <span className="ver-tag__mark">{mark}</span>}
    </label>
  );
}
