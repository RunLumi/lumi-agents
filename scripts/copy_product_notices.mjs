// SPDX-License-Identifier: Apache-2.0
// Product terms and existing inventories only; not a complete dependency SBOM.
import { copyFile, mkdir, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
export async function copyProductNotices(destination, surface) {
  if (!['web', 'desktop'].includes(surface)) throw new Error('Unknown distribution surface');
  const files = [['LICENSE', 'LICENSE.txt'], ['NOTICE', 'NOTICE.txt'],
    ['THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_NOTICES.md'],
    [surface === 'web' ? 'web/THIRD_PARTY_NOTICES.md' : 'apps/desktop/THIRD_PARTY_NOTICES.md', 'SURFACE_THIRD_PARTY_NOTICES.md']];
  await mkdir(destination, { recursive: true });
  for (const [source, name] of files) {
    await copyFile(join(root, source), join(destination, name));
    if (!(await readFile(join(root, source))).equals(await readFile(join(destination, name)))) {
      throw new Error('Product notice copy mismatch');
    }
  }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const surface = process.argv[2];
  if (!['web', 'desktop'].includes(surface)) throw new Error('Usage: node scripts/copy_product_notices.mjs web|desktop');
  const destination = join(root, surface === 'web' ? 'web/dist/legal' : 'apps/desktop/ui/dist/legal');
  await copyProductNotices(destination, surface);
  console.log(`Product license/notice files verified for ${surface}; dependency compliance remains separately required.`);
}
