# SPDX-License-Identifier: Apache-2.0
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from check_licensing import ROOT, check


class LicensingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        scope = json.loads((ROOT / 'docs/licensing/scope.json').read_text())
        files = ['LICENSE', 'NOTICE', 'DCO', 'LICENSING.md', 'TRADEMARKS.md', 'CONTRIBUTING.md',
                 'docs/licensing/scope.json', 'docs/licensing/commercial-and-rights-review.md',
                 'docs/adr/0012-community-rights-commercial-boundary.md', 'docs/adr/0003-licensing.md',
                 '.github/workflows/release.yml', 'apps/desktop/src-tauri/tauri.conf.json']
        files += scope['first_party_npm_manifests'] + scope['first_party_cargo_manifests'] + scope['third_party_notice_files']
        files += [str(Path(p).with_name('package-lock.json')) for p in scope['first_party_npm_manifests']
                  if (ROOT / Path(p).with_name('package-lock.json')).is_file()]
        for path in files:
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / path, target)

    def tearDown(self):
        self.temp.cleanup()

    def mutate_json(self, path, fn):
        p = self.root / path
        d = json.loads(p.read_text())
        fn(d)
        p.write_text(json.dumps(d))

    def test_real_repository_and_fixture_pass(self):
        self.assertEqual(check(), [])
        self.assertEqual(check(self.root), [])

    def test_changed_apache_terms_rejected(self):
        with (self.root / 'LICENSE').open('a') as f:
            f.write('\nNo commercial use.\n')
        self.assertTrue(any('LICENSE changed' in e for e in check(self.root)))

    def test_dco_text_not_rewritten(self):
        (self.root / 'DCO').write_text('I transfer copyright')
        self.assertTrue(any('DCO changed' in e for e in check(self.root)))

    def test_attribution_preserved(self):
        (self.root / 'NOTICE').write_text('A different exclusive owner')
        self.assertTrue(any('NOTICE changed' in e for e in check(self.root)))

    def test_npm_license_mismatch(self):
        self.mutate_json('apps/desktop/ui/package.json', lambda d: d.update(license='Elastic-2.0'))
        self.assertTrue(any('missing/contradictory license' in e for e in check(self.root)))

    def test_lock_root_mismatch(self):
        self.mutate_json('web/package-lock.json', lambda d: d['packages'][''].pop('license'))
        self.assertTrue(any('root lock license mismatch' in e for e in check(self.root)))

    def test_missing_previously_locked_graph_rejected(self):
        (self.root / 'web/package-lock.json').unlink()
        self.assertTrue(any('missing lock' in e for e in check(self.root)))

    def test_exception_cannot_hide_another_missing_lock(self):
        self.mutate_json('docs/licensing/scope.json', lambda d: d['baseline_unlocked_packages'].append(
            {'manifest': 'web/package.json', 'reason': 'skip it'}))
        self.assertTrue(any('lock exceptions' in e for e in check(self.root)))

    def test_baseline_gap_must_be_updated_when_lock_added(self):
        (self.root / 'workers/playwright/package-lock.json').write_text('{"packages":{"":{"license":"Apache-2.0"}}}')
        self.assertTrue(any('stale baseline lock exception' in e for e in check(self.root)))

    def test_publication_not_accidentally_enabled(self):
        self.mutate_json('web/package.json', lambda d: d.update(private=False))
        self.assertTrue(any('publication needs' in e for e in check(self.root)))

    def test_unlisted_package_rejected(self):
        p = self.root / 'extensions/new/package.json';p.parent.mkdir(parents=True)
        p.write_text('{"name":"new"}')
        self.assertTrue(any('inventory' in e for e in check(self.root)))

    def test_cargo_alternate_license_file_rejected(self):
        p = self.root / 'apps/desktop/src-tauri/Cargo.toml'
        p.write_text(p.read_text().replace('[package]', '[package]\nlicense-file = "PRIVATE"'))
        self.assertTrue(any('alternate license-file' in e for e in check(self.root)))

    def test_private_scope_not_fabricated(self):
        self.mutate_json('docs/licensing/scope.json', lambda d: d.update(commercial_modules_in_this_repository=['crates/lumi-policy']))
        self.assertTrue(any('commercial code' in e for e in check(self.root)))

    def test_missing_desktop_resource_rejected(self):
        self.mutate_json('apps/desktop/src-tauri/tauri.conf.json', lambda d: d['bundle']['resources'].pop('../../../NOTICE'))
        self.assertTrue(any('missing desktop resource' in e for e in check(self.root)))

    def test_direct_build_notice_omission_rejected(self):
        self.mutate_json('web/package.json', lambda d: d['scripts'].update(build='astro build'))
        self.assertTrue(any('direct build' in e for e in check(self.root)))

    def test_broken_local_reference_rejected(self):
        p = self.root / 'TRADEMARKS.md'
        p.write_text(p.read_text()+'\n[broken](absent.md)\n')
        self.assertTrue(any('broken link' in e for e in check(self.root)))

    def test_product_notice_copier_exact_bytes_both_surfaces(self):
        script = (ROOT / 'scripts/copy_product_notices.mjs').as_uri()
        for surface in ['web', 'desktop']:
            destination = self.root / ('out-'+surface)
            program = f'import {{copyProductNotices}} from {json.dumps(script)}; await copyProductNotices({json.dumps(str(destination))}, {json.dumps(surface)});'
            subprocess.run(['node', '--input-type=module', '-e', program], check=True, capture_output=True)
            self.assertEqual((destination/'LICENSE.txt').read_bytes(), (ROOT/'LICENSE').read_bytes())
            self.assertEqual((destination/'NOTICE.txt').read_bytes(), (ROOT/'NOTICE').read_bytes())
            self.assertTrue((destination/'SURFACE_THIRD_PARTY_NOTICES.md').is_file())

    def test_notice_copier_rejects_unknown_surface(self):
        script = ROOT / 'scripts/copy_product_notices.mjs'
        p = subprocess.run(['node', str(script), 'unknown'], capture_output=True)
        self.assertNotEqual(p.returncode, 0)


if __name__ == '__main__':
    unittest.main()
