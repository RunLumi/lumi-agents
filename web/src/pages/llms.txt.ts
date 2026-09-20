import { previewBuild } from '../data/site.mjs';
import { renderLlmsText } from '../data/discovery.mjs';

export const prerender = true;
export function GET() {
  return new Response(renderLlmsText({ preview: previewBuild }), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
}
