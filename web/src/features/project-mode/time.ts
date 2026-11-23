/** A time of day as the person's language writes it ("8:00" in Japanese, "8:00 AM" in English). */
export function formatUntil(iso: string, language: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleTimeString(language, { hour: 'numeric', minute: '2-digit' });
}
