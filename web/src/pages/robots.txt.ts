import { site, previewBuild } from '../data/site.mjs';
export const prerender = true;
export function GET() {
  return new Response(previewBuild ? 'User-agent: *\nDisallow: /\n' : `User-agent: *\nAllow: /\nSitemap: ${site.origin}/sitemap.xml\n`, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
}
