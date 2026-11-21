// Reads Detail sections from the stream. The sections of one task that are asked for in the same
// moment (the document and the history of a task that opens) share one request, as they would if
// the stream were one read; a section asked for alone gets a request of its own. Each section is
// still its own query: a part of the batch that is cancelled leaves the others running, and the
// request is aborted only when every part has been.
import type { DetailSection, DetailVersions } from '../api/contracts';
import { readDetail, StreamRejected } from './stream-reader';
import type { DataAdapter, ResourceKey, SectionDataMap } from './types';
import type { DetailEntry } from './detail-snapshot';

const ORDER: readonly DetailSection[] = ['document', 'history'];

interface Part {
  signal: AbortSignal;
  settled: boolean;
  resolve(entry: DetailEntry): void;
  reject(error: Error): void;
}

export interface DetailBatchOptions {
  adapter: DataAdapter;
  /** The versions the Summary names for sections (when it can be trusted), to hold a stream to. */
  expected(key: ResourceKey, sections: DetailSection[]): DetailVersions | undefined;
  /** What a running request has received of a section so far. */
  onPartial(key: ResourceKey, section: DetailSection, data: SectionDataMap[DetailSection]): void;
}

const aborted = () => new StreamRejected('aborted', 'the request was aborted');
const idOf = (key: ResourceKey) => `${key.kind}:${key.projectId}:${key.id}`;

export class DetailBatch {
  private readonly pending = new Map<
    string,
    { key: ResourceKey; parts: Map<DetailSection, Part> }
  >();

  constructor(private readonly options: DetailBatchOptions) {}

  /** Reads one section; a request for another section of the same Detail made now joins it. */
  read<S extends DetailSection>(
    key: ResourceKey,
    section: S,
    signal: AbortSignal,
  ): Promise<DetailEntry<S>> {
    return new Promise((resolve, reject) => {
      const id = idOf(key);
      let batch = this.pending.get(id);
      if (!batch) {
        batch = { key, parts: new Map() };
        this.pending.set(id, batch);
        queueMicrotask(() => this.flush(id));
      }
      batch.parts.set(section, {
        signal,
        settled: false,
        resolve: resolve as (entry: DetailEntry) => void,
        reject,
      });
    });
  }

  private flush(id: string): void {
    const batch = this.pending.get(id);
    this.pending.delete(id);
    if (!batch) return;
    const { key } = batch;
    const sections = ORDER.filter((s) => batch.parts.has(s));
    const parts = sections.map((s) => batch.parts.get(s) as Part);
    const controller = new AbortController();
    const settle = (part: Part, run: () => void) => {
      if (part.settled) return;
      part.settled = true;
      run();
      if (parts.every((p) => p.settled)) controller.abort();
    };
    for (const part of parts) {
      const abort = () => settle(part, () => part.reject(aborted()));
      if (part.signal.aborted) abort();
      else part.signal.addEventListener('abort', abort, { once: true });
    }
    if (parts.every((p) => p.settled)) return;
    readDetail(
      this.options.adapter.streamDetail({ resource: key, sections }, controller.signal),
      { resource: key, sections, versions: this.options.expected(key, sections) },
      {
        signal: controller.signal,
        onPartial: (section, data) => {
          if (!batch.parts.get(section)?.settled) this.options.onPartial(key, section, data);
        },
      },
    ).then(
      (result) => {
        sections.forEach((section, i) => {
          const version = result.versions[section];
          const data = result[section];
          settle(parts[i] as Part, () =>
            version === undefined || data === undefined
              ? (parts[i] as Part).reject(
                  new StreamRejected('incomplete', `no ${section} in the stream`),
                )
              : (parts[i] as Part).resolve({ data, version } as DetailEntry),
          );
        });
      },
      (error: Error) => {
        for (const part of parts) settle(part, () => part.reject(error));
      },
    );
  }
}
