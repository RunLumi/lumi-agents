#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Prospective DCO trailer check. Never creates certifications or rewrites commits."""
from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

IDENTITY = re.compile(r'^([^<>\r\n]+?)\s*<([^<>\s]+@[^<>\s]+)>$')
SHA = re.compile(r'^[0-9a-f]{40}$')


def identity(value: str) -> tuple[str, str] | None:
    match = IDENTITY.fullmatch(value.strip())
    if not match or not match[1].strip():
        return None
    return match[1].strip().casefold(), match[2].casefold()


def trailer_errors(author: str, parsed_trailers: str) -> list[str]:
    signed: set[tuple[str, str]] = set()
    authors = {identity(author)}
    errors: list[str] = []
    for line in parsed_trailers.splitlines():
        key, sep, value = line.partition(':')
        if not sep or key.casefold() not in {'signed-off-by', 'co-authored-by'}:
            continue
        person = identity(value)
        if person is None:
            errors.append('malformed author/sign-off identity')
        elif key.casefold() == 'signed-off-by':
            signed.add(person)
        else:
            authors.add(person)
    if None in authors:
        errors.append('invalid commit author identity')
    if not authors.issubset(signed):
        errors.append('commit author or named co-author lacks a matching DCO sign-off')
    return errors


def git(root: Path, *args: str, input: str | None = None) -> str:
    return subprocess.run(['git', '-C', str(root), *args], input=input, text=True,
                          check=True, capture_output=True).stdout


def check_range(root: Path, base: str, head: str) -> tuple[bool, list[str]]:
    if not SHA.fullmatch(base) or not SHA.fullmatch(head):
        raise ValueError('Expected full lowercase commit SHAs')
    git(root, 'cat-file', '-e', f'{base}^{{commit}}')
    git(root, 'cat-file', '-e', f'{head}^{{commit}}')
    policy = subprocess.run(['git', '-C', str(root), 'cat-file', '-e', f'{base}:DCO'], capture_output=True)
    if policy.returncode:
        return False, []  # Policy adoption only, never retroactive attestation.
    errors = []
    for commit in git(root, 'rev-list', '--no-merges', f'{base}..{head}').splitlines():
        author, body = git(root, 'show', '-s', '--format=%an <%ae>%x00%B', commit).split('\0', 1)
        parsed = git(root, 'interpret-trailers', '--parse', input=body)
        errors += [f'{commit[:12]}: {error}' for error in trailer_errors(author.strip(), parsed)]
    return True, errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True)
    parser.add_argument('--head', required=True)
    args = parser.parse_args()
    try:
        applied, errors = check_range(Path.cwd(), args.base, args.head)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f'DCO check could not run: {type(error).__name__}')
        return 1
    if not applied:
        print('DCO policy bootstrap: base has no DCO. No historical certification is asserted.')
        return 0
    for error in errors:
        print(error)
    if errors:
        print('Obtain genuine sign-offs from authorized contributors; never synthesize consent in CI.')
        return 1
    print('DCO trailer consistency passed; not proof of ownership, signature or employer authority.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
