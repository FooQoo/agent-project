/** What a screen that reads a project-scoped resource is given. */
export interface ProjectResourceResult<T> {
  status: 'loading' | 'ready' | 'error';
  data: T | null;
  error: 'not_found' | 'failed' | null;
  fetchFailed: boolean;
  lastSyncedAt: Date | null;
  reload: () => void;
}
