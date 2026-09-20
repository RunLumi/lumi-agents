import { defineConfig } from 'astro/config';

// This is a static Pages site, not an SSR Worker. No adapter or credentials.
export default defineConfig({
  site: 'https://agents.runlumi.app',
  output: 'static',
  trailingSlash: 'always',
  build: { inlineStylesheets: 'never' },
  vite: { build: { assetsInlineLimit: 0 } },
  devToolbar: { enabled: false },
});
