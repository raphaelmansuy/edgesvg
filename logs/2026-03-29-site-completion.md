# 2026-03-29 — Astro Site Completion

## Actions
- Fixed `content.config.ts` to use `docsLoader()` from `@astrojs/starlight/loaders` (required for Astro 5+)
- Created all remaining docs pages: `concepts/modes.md`, `api/rust.md`, `api/python.md`, `api/nodejs.md`, `api/cli.md`, `api/wasm.md`, `guides/batch-conversion.md`, `guides/cicd.md`, `changelog.mdx`
- Removed demo.mdx (MDX CSS blocks incompatible) — demo sidebar link now points to external GitHub Pages demo URL
- Removed demo.astro and DemoLayout.astro (not needed)
- Updated `astro.config.mjs` sidebar: `guides/demo` changed to external `link` with `↗`
- Created `.github/workflows/deploy-site.yml` for GitHub Pages deployment
- Added `site-dev`, `site-build`, `site-preview` targets to Makefile
- Build: 18 pages, clean, 3.5s

## Decisions
- Demo is an external link to `https://raphaelmansuy.github.io/edgesvg/demo/` per user request
- Starlight `link:` sidebar items used for external links; `slug:` for content collection pages

## Next steps
- Enable GitHub Pages in repo settings (source: GitHub Actions)
- Push to main to trigger first deploy
- Verify demo app is deployed to the demo/ subpath

## Lessons
- Astro 5 content collections require `loader: docsLoader()` — old `defineCollection({ schema })` without a loader yields empty collection
- MDX `<style>` blocks with CSS `{` trigger parse errors — move CSS to `<style is:global>` in `.astro` files or use external CSS
