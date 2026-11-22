// Mirrors docs/.vitepress/repo.ts: the site is served from the repository name, which
// GitHub Actions gives in GITHUB_REPOSITORY (owner/name). Local runs use the default.
const slug = process.env.GITHUB_REPOSITORY || 'FooQoo/agent-project';
const name = slug.split('/')[1] || 'agent-project';

export const siteBase = `/${name}/`;
export const siteBaseRegex = `\\/${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\/`;
