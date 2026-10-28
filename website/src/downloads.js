export const RELEASES_PAGE = 'https://github.com/ELY-Open-Source/ELYTerminal/releases';
export const RELEASE_ROOT = `${RELEASES_PAGE}/latest/download`;

export const ASSETS = Object.freeze({
  macos: 'ELYTerminal-macos-arm64.dmg',
  windows: 'ELYTerminal-windows-x86_64.zip',
  appimage: 'ELYTerminal-linux-x86_64.AppImage',
  deb: 'elyterminal_amd64.deb',
  rpm: 'elyterminal.x86_64.rpm',
  tarball: 'ELYTerminal-linux-x86_64.tar.gz',
});

const PRIMARY = { macos: 'macos', windows: 'windows', linux: 'appimage' };
const NAMES = { macos: 'macOS', windows: 'Windows', linux: 'Linux' };

export function releaseUrl(asset) {
  if (!Object.values(ASSETS).includes(asset)) throw new Error(`Downloads: unknown asset ${asset}`);
  return `${RELEASE_ROOT}/${asset}`;
}

export function detectPlatform({ userAgent = '', platform = '', userAgentData, maxTouchPoints = 0 } = {}) {
  const modern = typeof userAgentData?.platform === 'string' ? userAgentData.platform : '';
  const text = `${modern} ${platform} ${userAgent}`;
  if (/Android|iPhone|iPad|iPod|CrOS|Mobile/i.test(text)) return 'other';
  if (/MacIntel/i.test(platform) && maxTouchPoints > 1) return 'other';
  if (/Windows/i.test(text)) return 'windows';
  if (/macOS|MacIntel|Macintosh|Mac OS X/i.test(text)) return 'macos';
  if (/Linux/i.test(text)) return 'linux';
  return 'other';
}

export function createDownloads() {
  const platform = detectPlatform(navigator);
  const primary = document.getElementById('primary-download');
  const label = document.getElementById('primary-label');
  if (!primary || !label) throw new Error('Downloads: missing primary link');
  for (const link of document.querySelectorAll('[data-download]')) {
    const key = link.dataset.download;
    const asset = ASSETS[key];
    if (!asset) throw new Error(`Downloads: unknown key ${key}`);
    link.href = releaseUrl(asset);
  }
  primary.dataset.platform = platform;
  if (platform === 'other') {
    primary.href = RELEASES_PAGE;
    label.dataset.en = 'View releases';
    label.dataset.zh = '查看发布';
  } else {
    primary.href = releaseUrl(ASSETS[PRIMARY[platform]]);
    label.dataset.en = `Download for ${NAMES[platform]}`;
    label.dataset.zh = `下载 ${NAMES[platform]} 版`;
  }
  const locale = document.documentElement.dataset.locale;
  if (locale !== 'en' && locale !== 'zh') throw new Error(`Downloads: locale is "${locale}", set preferences first`);
  label.textContent = label.dataset[locale];
}
