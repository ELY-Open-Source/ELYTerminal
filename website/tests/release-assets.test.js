import assert from 'node:assert/strict';
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import test from 'node:test';

const root = path.resolve(import.meta.dirname, '../..');
const files = {
  'ELYTerminal-0.1.9.dmg': 'ELYTerminal-macos-arm64.dmg',
  'ELYTerminal-0.1.9-windows-x86_64.zip': 'ELYTerminal-windows-x86_64.zip',
  'ELYTerminal-0.1.9-linux-x86_64.AppImage': 'ELYTerminal-linux-x86_64.AppImage',
  'elyterminal_0.1.9_amd64.deb': 'elyterminal_amd64.deb',
  'elyterminal-0.1.9-1.x86_64.rpm': 'elyterminal.x86_64.rpm',
  'ELYTerminal-0.1.9-linux-x86_64.tar.gz': 'ELYTerminal-linux-x86_64.tar.gz',
};

async function harness() {
  await mkdir(path.join(root, 'target'), { recursive: true });
  const folder = await mkdtemp(path.join(root, 'target/release-alias-test-'));
  const bin = path.join(folder, 'bin');
  await mkdir(bin);
  await writeFile(path.join(folder, 'asset'), 'release');
  await writeFile(path.join(bin, 'cargo'), '#!/bin/sh\nprintf "%s\\n" "elyterminal#0.1.9"\n');
  await writeFile(path.join(bin, 'gh'), '#!/bin/sh\nprintf "%s\\n" "$*" >> "$GH_LOG"\n');
  await chmod(path.join(bin, 'cargo'), 0o755);
  await chmod(path.join(bin, 'gh'), 0o755);
  return {
    folder,
    env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, GH_LOG: path.join(folder, 'gh.log'), TMPDIR: folder },
  };
}

test('alias-only uploads exactly the six stable names', async () => {
  const { folder, env } = await harness();
  try {
    for (const [versioned, alias] of Object.entries(files)) {
      const file = path.join(folder, versioned);
      await writeFile(file, versioned);
      await writeFile(env.GH_LOG, '');
      const result = spawnSync('scripts/release-asset.sh', ['--alias-only', file], { cwd: root, env, encoding: 'utf8' });
      assert.equal(result.status, 0, result.stderr);
      const calls = (await readFile(env.GH_LOG, 'utf8')).trim().split('\n');
      assert.equal(calls.length, 1);
      assert.ok(calls[0].startsWith('release upload v0.1.9 '));
      assert.ok(calls[0].includes(`/${alias} --clobber`), calls[0]);
    }
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});

test('normal upload carries its alias and unknown aliases fail', async () => {
  const { folder, env } = await harness();
  try {
    const file = path.join(folder, 'ELYTerminal-0.1.9.dmg');
    await writeFile(file, 'dmg');
    const result = spawnSync('scripts/release-asset.sh', [file], { cwd: root, env, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    const calls = (await readFile(env.GH_LOG, 'utf8')).trim().split('\n');
    assert.equal(calls.length, 2);
    assert.ok(calls[0].includes('/ELYTerminal-0.1.9.dmg --clobber'));
    assert.ok(calls[1].includes('/ELYTerminal-macos-arm64.dmg --clobber'));
    const bad = spawnSync('scripts/release-asset.sh', ['--alias-only', path.join(folder, 'asset')], { cwd: root, env, encoding: 'utf8' });
    assert.notEqual(bad.status, 0);
    assert.match(bad.stderr, /no stable alias/);
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});
