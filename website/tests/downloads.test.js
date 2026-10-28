import assert from 'node:assert/strict';
import test from 'node:test';
import { ASSETS, RELEASES_PAGE, RELEASE_ROOT, detectPlatform, releaseUrl } from '../src/downloads.js';

test('platform detection covers desktop and unsupported mobile systems', () => {
  assert.equal(detectPlatform({ userAgentData: { platform: 'macOS' } }), 'macos');
  assert.equal(detectPlatform({ userAgentData: { platform: 'Windows' } }), 'windows');
  assert.equal(detectPlatform({ userAgentData: { platform: 'Linux' } }), 'linux');
  assert.equal(detectPlatform({ platform: 'MacIntel', userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)' }), 'macos');
  assert.equal(detectPlatform({ platform: 'MacIntel', userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)', maxTouchPoints: 5 }), 'other');
  assert.equal(detectPlatform({ platform: 'Win32', userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)' }), 'windows');
  assert.equal(detectPlatform({ platform: 'Linux x86_64', userAgent: 'X11; Linux x86_64' }), 'linux');
  assert.equal(detectPlatform({ userAgent: 'Android; Mobile; Linux' }), 'other');
  assert.equal(detectPlatform({ userAgent: 'iPad; Mac OS X; Mobile' }), 'other');
  assert.equal(detectPlatform({ userAgent: 'X11; CrOS x86_64' }), 'other');
  assert.equal(detectPlatform({ platform: 'FreeBSD' }), 'other');
});

test('all six release links are stable and versionless', () => {
  assert.deepEqual(Object.values(ASSETS), [
    'ELYTerminal-macos-arm64.dmg', 'ELYTerminal-windows-x86_64.zip',
    'ELYTerminal-linux-x86_64.AppImage', 'elyterminal_amd64.deb',
    'elyterminal.x86_64.rpm', 'ELYTerminal-linux-x86_64.tar.gz',
  ]);
  for (const asset of Object.values(ASSETS)) {
    assert.equal(releaseUrl(asset), `${RELEASE_ROOT}/${asset}`);
    assert.doesNotMatch(releaseUrl(asset), /0\.1\.9/);
  }
  assert.equal(RELEASES_PAGE, 'https://github.com/ELY-Open-Source/ELYTerminal/releases');
  assert.throws(() => releaseUrl('ELYTerminal-0.1.9.dmg'), /unknown asset/);
});
