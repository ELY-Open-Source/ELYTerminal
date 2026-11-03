import assert from 'node:assert/strict';
import test from 'node:test';
import { crateVersion, stampVersion, VERSION_MARK } from '../version.js';

const manifest = `[package]
name = "elyterminal"
version = "0.3.14"
edition = "2024"

[dependencies]
serde = { version = "1.0.228", features = ["derive"] }

[workspace.package]
version = "9.9.9"
`;

test('the version is the package section\'s', () => {
  assert.equal(crateVersion(manifest), '0.3.14');
  assert.equal(crateVersion(manifest.replace('0.3.14', '12.0.7')), '12.0.7');
});

test('a version outside the package section is not taken', () => {
  const later = '[dependencies]\nversion = "1.2.3"\n\n[package]\nname = "elyterminal"\nversion = "4.5.6"\n';
  assert.equal(crateVersion(later), '4.5.6');
  assert.throws(() => crateVersion('[dependencies]\nversion = "1.2.3"\n'), /no \[package\] version/);
});

test('a manifest without a plain version fails', () => {
  assert.throws(() => crateVersion('[package]\nname = "elyterminal"\n'), /no \[package\] version/);
  assert.throws(() => crateVersion('[package]\nversion.workspace = true\n'), /no \[package\] version/);
  assert.throws(() => crateVersion('[package]\nversion = "0.2"\n'), /not MAJOR\.MINOR\.PATCH/);
});

test('the page takes the version at every mark', () => {
  const html = `<p>Compiling elyterminal v${VERSION_MARK}</p><i>${VERSION_MARK}</i>`;
  assert.equal(stampVersion(html, '0.3.14'), '<p>Compiling elyterminal v0.3.14</p><i>0.3.14</i>');
});

test('a page without the mark fails', () => {
  assert.throws(() => stampVersion('<p>Compiling elyterminal v0.1.9</p>', '0.3.14'), /no version mark/);
});
