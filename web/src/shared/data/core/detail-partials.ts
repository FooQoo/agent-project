// What a running Detail stream has received so far, by query. A query holds only whole versions,
// so what arrives before `complete` is kept here, shown while the stream runs, and dropped when it
// ends (whatever way it ends).
import type { SectionDataMap } from './types';

export class DetailPartials {
  private readonly values = new Map<string, unknown>();
  private readonly listeners = new Set<() => void>();

  /** For `useSyncExternalStore`. */
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  get<S extends keyof SectionDataMap>(hash: string): SectionDataMap[S] | null {
    return (this.values.get(hash) as SectionDataMap[S] | undefined) ?? null;
  }

  set(hash: string, value: SectionDataMap[keyof SectionDataMap]): void {
    this.values.set(hash, value);
    this.emit();
  }

  clear(hash: string): void {
    if (this.values.delete(hash)) this.emit();
  }

  private emit(): void {
    for (const listener of [...this.listeners]) listener();
  }
}
