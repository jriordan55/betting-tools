// Same reason as the calculator route: with SSR off there is nothing for the
// prerenderer to crawl, so this is served by the SPA fallback at runtime.
export const prerender = false;
