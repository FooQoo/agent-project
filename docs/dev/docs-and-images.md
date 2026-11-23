# Documentation and images

The documentation site lives in `docs/`. This page explains how to preview and build it, and how to regenerate the screenshots and demo videos.

## Documentation site

The documentation site lives in `docs/` and is built with VitePress. It has the user guide (`docs/guide/`) and this developer chapter (`docs/dev/`). English is the source language; keep the Japanese pages in `docs/ja/` in step with every English change. It is published to GitHub Pages from `main` by `.github/workflows/docs.yml`; pull requests that change `docs/**`, `web/**`, or the demo suite only build and test it. The same workflow builds the [demo](./getting-started/setup.md#demo-build) after the site and copies `web/dist-demo/` into `docs/.vitepress/dist/demo/`, so it is served at `/<repository name>/demo/`. It then runs the demo suite against that output and stops before the upload if the suite fails; see [The browser demo](./concepts/demo.md#check-it-locally). Links to the demo from the guide use its full URL, because the demo is not a VitePress page.

```sh
pnpm --dir docs install
pnpm --dir docs dev --port <port>       # live preview
pnpm --dir docs build                   # also checks for dead links
pnpm --dir docs preview --port <port>   # serve the built site
```

Run `pnpm --dir docs build` before committing any change under `docs/`. Links to repository files outside `docs/` use absolute GitHub URLs.

The home page has no Markdown body. Its copy lives in the `landing` frontmatter of `docs/index.md` and `docs/ja/index.md`, and `docs/.vitepress/theme/LandingPage.vue` lays it out, picks the screenshots for the language, and swaps the board image for its dark version. Edit the copy in both files and keep their keys the same. The build's dead-link check does not see links in frontmatter, so the site tests open every link on the home page instead. Its colors are the `--lp-*` variables in `custom.css`, which follow the app's lane and pending colors.

The site and the demo are served from the repository name, so the base path follows it. `docs/.vitepress/repo.ts` and `web/demo-build.ts` read `GITHUB_REPOSITORY` (`owner/name`, set by GitHub Actions) and default to `FooQoo/agent-project` for local builds, and the site tests read it the same way through `e2e/site-base.ts`. The examples below use the default name.

The English top page (`/agent-project/`) chooses its language in the browser:

- A reader whose browser prefers Japanese (the first entry of `navigator.languages` starts with `ja`) is sent to `/agent-project/ja/`.
- A language picked in the nav's language menu, on a desktop or a phone, is saved in `localStorage` under `agent-project-docs-lang`. From then on the saved choice wins over the browser language.
- Only the English top page moves. Deep links, the Japanese pages, and in-site navigation never redirect. When storage is blocked, the choice is not kept and the browser language decides.

The redirect is client-side only, so the built HTML of every page stays the same for crawlers. `docs/.vitepress/theme/languageChoice.ts` holds both parts. `config.ts` inlines its redirect script right after the charset of the top page only, ahead of the stylesheets, so the English page barely shows before it moves. The script reads `base` from the config. The theme's `enhanceApp` saves the choice when a language menu link is clicked. The site tests in `e2e/docs/language.spec.ts` cover Japanese and English browsers, a saved choice, the phone menu, blocked storage, and deep links.

Keep diagrams focused on one flow. Flowcharts and sequence diagrams retain their label size; diagrams wider than the article scroll within their frame. Use a left-to-right flow for system diagrams, and split large diagrams by purpose. The site tests check label size and catch unparsed bold markers in rendered prose. Tables keep a readable minimum column width and scroll horizontally within the article; prose inside cells can still wrap.

Regenerate the guide screenshots and the demo videos with the scripts below. Both need the web UI and CLI [built](./getting-started/setup.md#build) first, and the Playwright browser from [Verification](./getting-started/pull-request.md#verification).

```sh
node e2e/scripts/docs-screenshots.mjs   # docs/images/guide/*.png (26 images, English and Japanese)
node e2e/scripts/demo-video.mjs         # docs/public/videos/ (also needs ffmpeg)
```

Both scripts use disposable data in a temporary `AGPJ_HOME` and their own loopback ports, so your saved board and port 7420 are untouched. Review the generated images and videos before committing them.

### Deterministic captures

Capturing twice from the same commit gives the same images, so a change in a picture means a change in the product. The three scripts (`readme-screenshots.mjs`, `docs-screenshots.mjs`, `demo-video.mjs`) share the settings in `e2e/scripts/lib/sample-board.mjs`:

- **One fixed "now".** `FIXED_NOW` (`2026-10-15T09:41:00Z`) is exported to the CLI and the server as `AGPJ_FIXED_NOW`. Product code takes its time from `crate::clock::now()`, which returns that instant when the variable holds a UTC timestamp, so every timestamp the sample data writes is the same. The dates the board reads from a task file's modified time follow it too. The variable has no effect when it is unset or does not parse.
- **The browser has the same clock.** `freezeClock(context)` calls `context.clock.setFixedTime`, so `Date.now()` and `new Date()` return the same instant in every page. The "Synced" time and the card dates agree with the data. Timers still run. Contexts also use the UTC time zone.
- **Rendering is steady.** Chromium starts with `--disable-gpu --disable-lcd-text --font-render-hinting=none`. `settle()` holds repeating animations (a spinner) at their first frame, and then waits until three screenshots in a row match, so a fade or a carousel that is still moving is not captured.

A screenshot taken after these steps matches the previous one pixel for pixel, apart from a few edge pixels in some phone and tablet images that differ by 1 or 2 levels in a channel (anti-aliasing of a rounded border). A recorded video follows the wall clock, so its frames shift a little from run to run and are compared with a tolerance.

### Compare captures with the committed files

`e2e/scripts/compare-captures.mjs` reports which captured files differ in what they show from the files committed in the repository. Point `AGPJ_MEDIA_OUT` at a temporary folder for the capture, then compare:

```sh
AGPJ_MEDIA_OUT=/tmp/captures node e2e/scripts/readme-screenshots.mjs
AGPJ_MEDIA_OUT=/tmp/captures node e2e/scripts/docs-screenshots.mjs
AGPJ_MEDIA_OUT=/tmp/captures node e2e/scripts/demo-video.mjs
node e2e/scripts/compare-captures.mjs --captured /tmp/captures            # list the changed files
node e2e/scripts/compare-captures.mjs --captured /tmp/captures --json     # {"changed":[{"file","status","target"}],"unchanged":N}
node e2e/scripts/compare-captures.mjs --captured /tmp/captures --apply    # copy the changed files into place
```

`readme/` is compared with `docs/images/`, `guide/` with `docs/images/guide/`, and `videos/` with `docs/public/videos/`. The exit code is 0 when nothing differs, 1 when something does, and 2 for a usage error, an unreadable file, or a folder with no PNG or MP4 files. A new file counts as changed (`added`). `--apply` also copies the `.jpg` poster of a changed video.

- A PNG is decoded to RGBA and compared by its pixels, so encoder metadata and compression do not count. `--image-tolerance <n>` lets a pixel differ by up to `n` in a channel; use it (for example `4`) to ignore edge anti-aliasing noise.
- An MP4 is decoded by `ffmpeg` to one frame per second at 480 x 300. Identical frames are unchanged. Otherwise each frame is compared with the frames a second either side of it, and the video is changed only when more than a tenth of its frames differ in more than 0.3% of their pixels. A change smaller than that, such as a date in a card, is not noticed in a video; the screenshots show it.
- `--against <dir>` compares with another `AGPJ_MEDIA_OUT` folder instead of the repository, which is how to check that two captures match.

## Move a page and keep its old URL

Old guide URLs are published in the README, the plugin README, and other people's links, so a page that moves must keep answering at its old address. `docs/.vitepress/redirects.ts` holds the list of moved pages as pairs of an old path and a new path, without a leading slash and without an extension, for example `guide/use/board` and `guide/watch/board`. For each pair the build writes a small HTML page at the old path that forwards to the new page.

- When you move or rename a page, add one entry to the list, and the file expands it into the English and the Japanese path (`ja/...`). Move the Japanese page in the same change.
- The build fails when a target page was not built, so a typo in the map is caught before it is published. It also fails when an old path is still a real page, so delete the old file when you add the entry.
- The redirect pages are written after the search index is built, so they do not appear in search.

## Check published release notes

The site header shows no version. Before the build, `node scripts/release/docs-release.mjs` reads the published GitHub Releases and checks their notes, so a malformed Release stops the build.

The release notes live in two places:

- The GitHub Release body, in English. `releases/vX.Y.Z.md` is its review copy.
- The site's release notes page, `docs/releases.md` and its Japanese translation `docs/ja/releases.md`, with every published version newest first. Add the new version to both pages when its Release is published; the script does not write them.

```sh
node scripts/release/docs-release.mjs --github --repo FooQoo/agent-project   # the published site
node scripts/release/docs-release.mjs --fixture                              # local preview of a published state
node scripts/release/docs-release.mjs --none                                 # the state before any release (default)
pnpm --dir docs build
```

- It counts only releases with `draft=false`, `prerelease=false`, and a `vMAJOR.MINOR.PATCH` tag, and takes the newest by version. The version in `Cargo.toml` is never used. The release notes are written in English only. Each release body needs the four headings `## Changes`, `## Fixes`, `## Breaking changes`, and `## Required update steps`, each exactly once; a missing or repeated heading stops the script. Keep a heading that has nothing to report and write `None.` under it.
- It writes `docs/.vitepress/release.json` (ignored by Git) with the state it found. `--fixture` uses the review copies in `releases/` and records the fixture state.
- The Pages workflow runs the script with `--github` before the build, on `main` pushes, on `release` events (published, edited, deleted), and by hand. If the API call fails or a release is invalid, the build stops and the published site stays as it is. Pull requests build offline with the committed state. A release published with the workflow's own token may not start the workflow; run it by hand.
- `releases/vX.Y.Z.md` is the review copy of the release notes. The GitHub Release body is the source of truth once it is published.
- To check a Release body before you publish it, run the script with `--releases-json` and a file in the shape of the GitHub API list.

## UI stories and coverage

Use Storybook to isolate a component state or compose a full page with the same UI parts the application uses. Keep the Story, focused test, dedicated stylesheet, and coverage entry with the source. When a component or page changes, update its Story and visible-state entry, then review the rendered page at the locale, theme, and viewport it represents. The Storybook page in [Set up and run](./getting-started/setup.md#storybook) describes the current automated coverage limits.

## README screenshots

After [building the web UI and CLI](./getting-started/setup.md#build), regenerate the README images:

```sh
pnpm --dir e2e install --frozen-lockfile
pnpm --dir e2e exec playwright install chromium
node e2e/scripts/readme-screenshots.mjs
```

The script creates disposable projects in temporary `AGPJ_HOME` directories, starts its own servers on available loopback ports, and captures the desktop board, the epic progress panel, the board at a tablet width (1024 x 768, behind the remote entrance), and four phone screens (the Doing lane with its tabs, a chosen card slid open to show its buttons, a question with a choice picked, and the bottom drawer) in English and Japanese. It never uses your saved board or port 7420. The browser renders the built UI against the real API; images are not mockups. It checks the four detail tabs, the task editor's folded conditions panel, and browser errors, and removes its temporary data afterwards.

Review all fourteen images in `docs/images/` before committing them, and keep the sample text free of personal data. Screenshots do not replace the regression suite or phone keyboard checks. The guide images in `docs/images/guide/` (26 files) come from the separate script above; review them too when you regenerate them.
