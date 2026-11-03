export const VERSION_MARK = '__ELY_VERSION__';

export function crateVersion(manifest) {
  let inPackage = false;
  for (const raw of manifest.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith('[')) {
      inPackage = line === '[package]';
      continue;
    }
    const match = inPackage && line.match(/^version\s*=\s*"([^"]*)"/);
    if (!match) continue;
    if (!/^\d+\.\d+\.\d+$/.test(match[1])) {
      throw new Error(`website: Cargo.toml's [package] version "${match[1]}" is not MAJOR.MINOR.PATCH`);
    }
    return match[1];
  }
  throw new Error("website: Cargo.toml has no [package] version");
}

export function stampVersion(html, version) {
  if (!html.includes(VERSION_MARK)) throw new Error(`website: index.html has no version mark ${VERSION_MARK}`);
  return html.replaceAll(VERSION_MARK, version);
}
