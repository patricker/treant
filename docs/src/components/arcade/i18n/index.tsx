import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type JSX,
  type ReactNode,
} from 'react';

// Runtime i18n for the arcade island. One site build, dictionaries switched
// live. Keys are gettext-style: the English string (with {param} placeholders)
// IS the key, so `t()` falls back to English automatically and `en` ships no
// dictionary at all. Non-English locales are flat JSON maps lazy-loaded as
// webpack chunks (precached by the PWA service worker like any other chunk).
export const LOCALES = [
  { id: 'en', native: 'English', flag: '🇺🇸' },
  { id: 'es', native: 'Español', flag: '🇪🇸' },
  { id: 'fr', native: 'Français', flag: '🇫🇷' },
  { id: 'de', native: 'Deutsch', flag: '🇩🇪' },
  { id: 'pt-BR', native: 'Português', flag: '🇧🇷' },
  { id: 'ja', native: '日本語', flag: '🇯🇵' },
  { id: 'zh-Hans', native: '简体中文', flag: '🇨🇳' },
] as const;

export type LocaleId = (typeof LOCALES)[number]['id'];
type Dict = Record<string, string>;

const KEY = 'treant-arcade-lang';
const isLocale = (v: unknown): v is LocaleId => LOCALES.some((l) => l.id === v);

// Static import() calls per locale so webpack can code-split each dictionary.
const LOADERS: Record<Exclude<LocaleId, 'en'>, () => Promise<{ default: Dict }>> = {
  es: () => import('./locales/es.json'),
  fr: () => import('./locales/fr.json'),
  de: () => import('./locales/de.json'),
  'pt-BR': () => import('./locales/pt-BR.json'),
  ja: () => import('./locales/ja.json'),
  'zh-Hans': () => import('./locales/zh-Hans.json'),
};

/** Best-match the browser language against our locale set (first visit only). */
function detectLocale(): LocaleId {
  try {
    for (const lang of navigator.languages ?? [navigator.language]) {
      if (!lang) continue;
      const exact = LOCALES.find((l) => l.id.toLowerCase() === lang.toLowerCase());
      if (exact) return exact.id;
      const prefix = lang.split('-')[0].toLowerCase();
      if (prefix === 'zh') return 'zh-Hans';
      if (prefix === 'pt') return 'pt-BR';
      const byPrefix = LOCALES.find((l) => l.id === prefix);
      if (byPrefix) return byPrefix.id;
    }
  } catch {
    /* SSR / privacy mode — default to English */
  }
  return 'en';
}

function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (m, name) =>
    name in params ? String(params[name]) : m,
  );
}

// Module-level mirror of the active dictionary so non-React code (game handles
// like 2048's statusText/endText) can translate too. The provider keeps it in
// sync; before the provider mounts it behaves as English passthrough.
let currentDict: Dict | null = null;

/** Non-hook translate for code outside the React tree (game handles). */
export function translate(key: string, params?: Record<string, string | number>): string {
  return interpolate(currentDict?.[key] ?? key, params);
}

export interface I18n {
  locale: LocaleId;
  setLocale: (l: LocaleId) => void;
  /** Translate an English template, then substitute `{param}` placeholders. */
  t: (key: string, params?: Record<string, string | number>) => string;
  /** Simple two-form plural: picks the English form by count, then translates. */
  tn: (
    n: number,
    singular: string,
    plural: string,
    params?: Record<string, string | number>,
  ) => string;
}

const LocaleContext = createContext<I18n>({
  locale: 'en',
  setLocale: () => {},
  t: (key, params) => interpolate(key, params),
  tn: (n, singular, plural, params) =>
    interpolate(n === 1 ? singular : plural, { n, ...params }),
});

export function LocaleProvider({ children }: { children: ReactNode }): JSX.Element {
  const [locale, setLocaleState] = useState<LocaleId>('en');
  const [dict, setDict] = useState<Dict | null>(null);

  // Resolve the initial locale once on mount: ?lang= beats the saved choice,
  // which beats browser-language detection. (Client-only: the arcade renders
  // inside <BrowserOnly>.)
  useEffect(() => {
    let initial: LocaleId | null = null;
    try {
      const fromUrl = new URLSearchParams(window.location.search).get('lang');
      if (isLocale(fromUrl)) initial = fromUrl;
    } catch {
      /* ignore */
    }
    if (!initial) {
      try {
        const saved = localStorage.getItem(KEY);
        if (isLocale(saved)) initial = saved;
      } catch {
        /* private mode — fall through to detection */
      }
    }
    setLocaleState(initial ?? detectLocale());
  }, []);

  // Load (or clear) the dictionary whenever the locale changes.
  useEffect(() => {
    if (locale === 'en') {
      setDict(null);
      currentDict = null;
      return;
    }
    let cancelled = false;
    LOADERS[locale]()
      .then((mod) => {
        if (!cancelled) {
          setDict(mod.default);
          currentDict = mod.default;
        }
      })
      .catch(() => {
        if (!cancelled) {
          setDict(null); // missing/broken dict → English fallback
          currentDict = null;
        }
      });
    return () => {
      cancelled = true;
    };
  }, [locale]);

  const setLocale = useCallback((l: LocaleId) => {
    setLocaleState(l);
    try {
      localStorage.setItem(KEY, l);
    } catch {
      /* ignore persistence failures */
    }
  }, []);

  const t = useCallback(
    (key: string, params?: Record<string, string | number>) =>
      interpolate(dict?.[key] ?? key, params),
    [dict],
  );

  const tn = useCallback(
    (
      n: number,
      singular: string,
      plural: string,
      params?: Record<string, string | number>,
    ) => {
      // The dictionary may override plural selection by translating both forms.
      const key = n === 1 ? singular : plural;
      return interpolate(dict?.[key] ?? key, { n, ...params });
    },
    [dict],
  );

  const value = useMemo(
    () => ({ locale, setLocale, t, tn }),
    [locale, setLocale, t, tn],
  );

  return <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>;
}

/** Access the arcade's translation helpers. Safe default (English) outside the provider. */
export function useT(): I18n {
  return useContext(LocaleContext);
}
