/** Opens an editor with what a loader returned; a loader that rejects or finds nothing opens none. */
export function openWhenLoaded<T>(load: Promise<T | null>, open: (loaded: T) => void): void {
  void load.then((loaded) => loaded && open(loaded)).catch(() => {});
}
