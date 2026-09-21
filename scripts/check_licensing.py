#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Offline first-party license and notice checks; not a legal/SBOM certification."""
from __future__ import annotations

import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
IGNORED = {'.git', 'node_modules', 'target', 'dist', '.astro', '.venv', '__pycache__'}


def inventory(root: Path, name: str) -> list[str]:
    return sorted(str(p.relative_to(root)) for p in root.rglob(name)
                  if not IGNORED.intersection(p.relative_to(root).parts))


def check(root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    scope = json.loads((root / 'docs/licensing/scope.json').read_text())

    def require(ok: bool, message: str) -> None:
        if not ok:
            errors.append(message)

    require(scope.get('schema_version') == 1, 'unknown licensing scope version')
    require(scope.get('default_license') == 'Apache-2.0', 'community license must remain explicit')
    require(scope.get('commercial_modules_in_this_repository') == [],
            'commercial code requires separate rights/repository review')
    for path, key in [('LICENSE', 'license_sha256'), ('NOTICE', 'notice_sha256'), ('DCO', 'dco_sha256')]:
        file = root / path
        require(file.is_file() and not file.is_symlink(), f'missing or linked {path}')
        if file.is_file() and not file.is_symlink():
            require(hashlib.sha256(file.read_bytes()).hexdigest() == scope.get(key),
                    f'{path} changed: explicit text/attribution review required')

    # Missing locks are not accepted for first-party packages. Keep this field
    # for backwards-compatible scope files, but require it to be empty after a
    # package has been reviewed and locked.
    unlocked = scope.get('baseline_unlocked_packages', [])
    expected_unlocked: list[str] = []
    require(isinstance(unlocked, list) and all(isinstance(x, dict) for x in unlocked),
            'invalid baseline lock exceptions')
    if not isinstance(unlocked, list) or not all(isinstance(x, dict) for x in unlocked):
        unlocked = []
    require([x.get('manifest') for x in unlocked] == expected_unlocked,
            'baseline lock exceptions require explicit review')
    for entry in unlocked:
        require(entry.get('baseline_commit') == scope.get('baseline_commit') and
                isinstance(entry.get('reason'), str) and bool(entry['reason'].strip()),
                'baseline lock exception must identify evidence and reason')

    for filename, key in [('package.json', 'first_party_npm_manifests'), ('Cargo.toml', 'first_party_cargo_manifests')]:
        found = inventory(root, filename)
        require(found == scope.get(key), f'{filename} scope inventory does not match actual packages')
        for path in found:
            file = root / path
            require(not file.is_symlink(), f'{path}: package manifest may not be a symlink')
            if file.is_symlink():
                continue
            if filename == 'package.json':
                data = json.loads(file.read_text())
                require(data.get('license') == 'Apache-2.0', f'{path}: missing/contradictory license')
                require(data.get('private') is True, f'{path}: publication needs an explicit release review')
                require('licenses' not in data, f'{path}: conflicting legacy licenses field')
                lock = file.with_name('package-lock.json')
                require(lock.is_file(), f'{path}: missing lock')
                if lock.is_file():
                    lockdata = json.loads(lock.read_text())
                    require(lockdata.get('packages', {}).get('', {}).get('license') == 'Apache-2.0',
                            f'{path}: root lock license mismatch')
            else:
                data = tomllib.loads(file.read_text())
                if path == 'Cargo.toml':
                    package = data.get('workspace', {}).get('package', {})
                    require(package.get('license') == 'Apache-2.0', 'workspace license mismatch')
                    require(package.get('publish') is False, 'workspace publication needs release review')
                else:
                    package = data.get('package', {})
                    expected = {'workspace': True} if path.startswith('crates/') else 'Apache-2.0'
                    require(package.get('license') == expected, f'{path}: license mismatch')
                require('license-file' not in package, f'{path}: alternate license-file requires review')

    for path in scope.get('third_party_notice_files', []):
        require((root / path).is_file(), f'missing third-party inventory: {path}')
    conf_path = root / 'apps/desktop/src-tauri/tauri.conf.json'
    conf = json.loads(conf_path.read_text())
    bundle = conf.get('bundle', {})
    require(bundle.get('license') == 'Apache-2.0', 'desktop bundle license mismatch')
    expected = {'../../../LICENSE': 'legal/LICENSE.txt', '../../../NOTICE': 'legal/NOTICE.txt',
                '../../../THIRD_PARTY_NOTICES.md': 'legal/THIRD_PARTY_NOTICES.md',
                '../THIRD_PARTY_NOTICES.md': 'legal/DESKTOP_THIRD_PARTY_NOTICES.md'}
    resources = bundle.get('resources', {})
    require(isinstance(resources, dict), 'desktop license resources require an explicit map')
    for source, target in expected.items():
        require(isinstance(resources, dict) and resources.get(source) == target,
                f'missing desktop resource: {source}')
        require((conf_path.parent / source).is_file(), f'missing desktop resource input: {source}')
    for surface, prefix in [('web', '../'), ('apps/desktop/ui', '../../../')]:
        manifest = json.loads((root / surface / 'package.json').read_text())
        require(f'node {prefix}scripts/copy_product_notices.mjs' in manifest.get('scripts', {}).get('build', ''),
                f'{surface}: direct build must preserve product notices')
    release = (root / '.github/workflows/release.yml').read_text()
    for source in ['LICENSE', 'NOTICE', 'THIRD_PARTY_NOTICES.md', 'apps/desktop/THIRD_PARTY_NOTICES.md']:
        require(f'cp {source} release-artifacts/' in release, f'release omits {source}')

    # Check only this policy's local references; links do not prove legal assertions.
    docs = ['LICENSING.md', 'TRADEMARKS.md', 'CONTRIBUTING.md',
            'docs/adr/0012-community-rights-commercial-boundary.md',
            'docs/licensing/commercial-and-rights-review.md']
    for path in docs:
        text = (root / path).read_text()
        for target in re.findall(r'\[[^\]]+\]\(([^\s)]+)\)', text):
            if ':' in target or target.startswith('#'):
                continue
            require(((root / path).parent / target.split('#')[0]).exists(), f'{path}: broken link {target}')
    return errors


def main() -> int:
    try:
        errors = check()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'Licensing check failed: {error}', file=sys.stderr)
        return 1
    for error in errors:
        print(f'FAIL: {error}', file=sys.stderr)
    if errors:
        return 1
    print(f'PASS: {len(inventory(ROOT, "Cargo.toml"))} Rust manifests, '
          f'{len(inventory(ROOT, "package.json"))} npm manifests; Apache community scope, text and notice wiring.')
    print('All first-party npm manifests have committed locks; dependency certification remains separate from this metadata check.')
    print('Evidence: metadata and packaging contracts only; no legal-title, trademark or complete SBOM certification.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
