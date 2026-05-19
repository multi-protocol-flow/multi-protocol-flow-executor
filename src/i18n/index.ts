import TypesafeI18n from './i18n-react';
import { i18n, isLocale, baseLocale, locales } from './i18n-util';
import { loadLocale } from './i18n-util.sync';
import type { Locales } from './i18n-types';

// 同步预加载所有 locale 的翻译（消除异步加载竞态）
loadLocale('zh-CN');
loadLocale('en-US');

// Detect initial locale
function detectInitialLocale(): Locales {
  try {
    const saved = localStorage.getItem('locale');
    if (saved && isLocale(saved)) {
      return saved as Locales;
    }
  } catch {}

  if (typeof navigator !== 'undefined' && navigator.language?.startsWith('zh')) {
    return 'zh-CN';
  }
  return baseLocale;
}

// Initialize i18n（仅用于首次启动，无历史记录时）
export function initI18n(): void {
  const detected = detectInitialLocale();
  switchLocale(detected);
}

// Switch locale（翻译已预加载，仅更新 localStorage）
export function switchLocale(newLocale: Locales): void {
  localStorage.setItem('locale', newLocale);
}

export { TypesafeI18n, i18n, baseLocale, locales, isLocale };
export type { Locales };

