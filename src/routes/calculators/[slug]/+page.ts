// The slug set lives in `src/lib/calculators.ts`, but with SSR off there is
// nothing for the prerenderer to crawl — it would render an empty page and
// find no links. This route is served by the SPA fallback at runtime instead.
export const prerender = false;
