#!/usr/bin/env python3
"""Validate release identity without importing development dependencies."""
import os, pathlib, re, tomllib
root = pathlib.Path(__file__).resolve().parents[1]
version = tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
assert re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', version), 'stable version required'
tag = os.environ.get('GITHUB_REF_NAME', '')
if os.environ.get('GITHUB_REF_TYPE') == 'tag':
    assert tag == 'v' + version, 'tag differs from Cargo package'
assert f'## [{version}]' in (root/'CHANGELOG.md').read_text(), 'changelog release missing'
print('release version:', version)
