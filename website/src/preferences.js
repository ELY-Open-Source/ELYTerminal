const LANGUAGE_KEY = 'elyterminal-language';
const THEME_KEY = 'elyterminal-theme';

export function detectLocale(language) {
  return /^zh(?:-|$)/i.test(language) ? 'zh' : 'en';
}

export function resolveTheme(preference, systemTheme) {
  return preference === 'light' || preference === 'dark' ? preference : systemTheme;
}

function readChoice(key) {
  try {
    return localStorage.getItem(key);
  } catch (error) {
    console.warn(`ELYTerminal: cannot read ${key}`, error);
    return null;
  }
}

function saveChoice(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch (error) {
    console.warn(`ELYTerminal: cannot save ${key}`, error);
  }
}

export function createPreferences() {
  const root = document.documentElement;
  const languageButton = document.getElementById('language-toggle');
  const themeButton = document.getElementById('theme-toggle');
  const themeColor = document.querySelector('meta[name="theme-color"]');
  if (!languageButton || !themeButton || !themeColor) {
    throw new Error('Preferences: missing controls');
  }
  const colorScheme = matchMedia('(prefers-color-scheme: light)');
  const systemTheme = () => (colorScheme.matches ? 'light' : 'dark');
  const savedTheme = readChoice(THEME_KEY);
  const savedLocale = readChoice(LANGUAGE_KEY);
  let followsSystem = savedTheme !== 'light' && savedTheme !== 'dark';
  let theme = resolveTheme(savedTheme, systemTheme());
  let locale = savedLocale === 'en' || savedLocale === 'zh'
    ? savedLocale
    : detectLocale(navigator.language);

  function applyLocale() {
    root.dataset.locale = locale;
    root.lang = locale === 'zh' ? 'zh-CN' : 'en';
    for (const element of document.querySelectorAll('[data-en][data-zh]')) {
      element.textContent = element.dataset[locale];
    }
    for (const element of document.querySelectorAll('[data-en-aria][data-zh-aria]')) {
      element.setAttribute('aria-label', element.dataset[`${locale}Aria`]);
    }
    languageButton.textContent = locale === 'zh' ? 'EN' : '中文';
    languageButton.setAttribute('aria-label', locale === 'zh' ? '切换到英语' : 'Switch to Simplified Chinese');
    document.dispatchEvent(new CustomEvent('elyterminal:locale'));
  }

  function applyTheme() {
    root.dataset.theme = theme;
    themeColor.content = theme === 'light' ? '#f1efe9' : '#0c0c0d';
    const label = theme === 'dark'
      ? ['Switch to light theme', '切换到浅色主题']
      : ['Switch to dark theme', '切换到深色主题'];
    themeButton.setAttribute('aria-label', label[locale === 'zh' ? 1 : 0]);
  }

  function apply() {
    applyLocale();
    applyTheme();
  }

  languageButton.addEventListener('click', () => {
    locale = locale === 'zh' ? 'en' : 'zh';
    saveChoice(LANGUAGE_KEY, locale);
    apply();
  });
  themeButton.addEventListener('click', () => {
    theme = theme === 'dark' ? 'light' : 'dark';
    followsSystem = false;
    saveChoice(THEME_KEY, theme);
    applyTheme();
  });
  colorScheme.addEventListener('change', () => {
    if (!followsSystem) return;
    theme = systemTheme();
    applyTheme();
  });
  apply();
}
