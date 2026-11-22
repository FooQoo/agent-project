// GitHub Pages serves the demo inside the documentation site, at /<repository name>/demo/.
// GitHub Actions sets GITHUB_REPOSITORY (owner/name); `AGPJ_DEMO_BASE` serves it from another path.
const repoName =
  (process.env.GITHUB_REPOSITORY || 'FooQoo/agent-project').split('/')[1] || 'agent-project';
export const DEMO_BASE = process.env.AGPJ_DEMO_BASE ?? `/${repoName}/demo/`;
export const DEMO_OUT_DIR = 'dist-demo';
