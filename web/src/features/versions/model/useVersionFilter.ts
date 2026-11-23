import { useState } from 'react';

/**
 * The version a screen filters its board by. It is that screen's own state: it starts as the project's
 * release version (or All when none is set) until a choice is made, and a choice made in one project
 * does not carry to another. `null` is "All".
 */
export function useVersionFilter(projectId: string | null, release: string | null) {
  const [chosen, setChosen] = useState<{ project: string | null; version: string | null } | null>(
    null,
  );
  const filter = chosen !== null && chosen.project === projectId ? chosen.version : release;
  return {
    filter,
    setFilter: (version: string | null) => setChosen({ project: projectId, version }),
  };
}
