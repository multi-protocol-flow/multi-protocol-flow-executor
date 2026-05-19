import { i18nObject, baseLocale, isLocale } from './i18n-util';
import type { Locales, TranslationFunctions } from './i18n-types';

let _LL: TranslationFunctions | null = null;
let _currentLocale: Locales = baseLocale;

function detectStoredLocale(): Locales {
  try {
    const saved = localStorage.getItem('locale');
    if (saved && isLocale(saved)) return saved;
  } catch {}
  return baseLocale;
}

export const getLL = () => {
  const locale = detectStoredLocale();
  if (!_LL || _currentLocale !== locale) {
    _LL = i18nObject(locale);
    _currentLocale = locale;
  }
  return _LL;
};
