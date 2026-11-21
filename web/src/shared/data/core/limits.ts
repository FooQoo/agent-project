// Timings and counts of the data core, in one place. Measure before changing them.
/** How long a change is collected before the fetches start. */
export const REFRESH_WINDOW_MS = 150;
/** A connection with no sign of life for this long is dropped and reopened. */
export const HEARTBEAT_DEADLINE_MS = 45_000;
export const RETRY_DELAYS_MS = [1_000, 2_000, 5_000, 10_000, 30_000] as const;
/** Fetches repeated after a response that contradicts a notification or the requested version. */
export const MAX_REFETCH_ATTEMPTS = 3;
