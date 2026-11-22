// The site is served from the repository name on GitHub Pages. GitHub Actions sets
// GITHUB_REPOSITORY (owner/name); local builds use the default.
const slug = process.env.GITHUB_REPOSITORY || 'FooQoo/agent-project'
const name = slug.split('/')[1] || 'agent-project'

export const repo = `https://github.com/${slug}`
export const base = `/${name}/`
