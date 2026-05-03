export interface TranslatorLanguage {
  code: string;
  name: string;
}

const ISO_639_1_CODES = [
  'aa', 'ab', 'ae', 'af', 'ak', 'am', 'an', 'ar', 'as', 'av', 'ay', 'az',
  'ba', 'be', 'bg', 'bh', 'bi', 'bm', 'bn', 'bo', 'br', 'bs',
  'ca', 'ce', 'ch', 'co', 'cr', 'cs', 'cu', 'cv', 'cy',
  'da', 'de', 'dv', 'dz',
  'ee', 'el', 'en', 'eo', 'es', 'et', 'eu',
  'fa', 'ff', 'fi', 'fj', 'fo', 'fr', 'fy',
  'ga', 'gd', 'gl', 'gn', 'gu', 'gv',
  'ha', 'he', 'hi', 'ho', 'hr', 'ht', 'hu', 'hy', 'hz',
  'ia', 'id', 'ie', 'ig', 'ii', 'ik', 'io', 'is', 'it', 'iu',
  'ja', 'jv',
  'ka', 'kg', 'ki', 'kj', 'kk', 'kl', 'km', 'kn', 'ko', 'kr', 'ks', 'ku', 'kv', 'kw', 'ky',
  'la', 'lb', 'lg', 'li', 'ln', 'lo', 'lt', 'lu', 'lv',
  'mg', 'mh', 'mi', 'mk', 'ml', 'mn', 'mr', 'ms', 'mt', 'my',
  'na', 'nb', 'nd', 'ne', 'ng', 'nl', 'nn', 'no', 'nr', 'nv', 'ny',
  'oc', 'oj', 'om', 'or', 'os',
  'pa', 'pi', 'pl', 'ps', 'pt',
  'qu',
  'rm', 'rn', 'ro', 'ru', 'rw',
  'sa', 'sc', 'sd', 'se', 'sg', 'si', 'sk', 'sl', 'sm', 'sn', 'so', 'sq', 'sr', 'ss', 'st', 'su', 'sv', 'sw',
  'ta', 'te', 'tg', 'th', 'ti', 'tk', 'tl', 'tn', 'to', 'tr', 'ts', 'tt', 'tw', 'ty',
  'ug', 'uk', 'ur', 'uz',
  've', 'vi', 'vo',
  'wa', 'wo',
  'xh',
  'yi', 'yo',
  'za', 'zh', 'zu',
];

const LANGUAGE_ALIASES: Record<string, string> = {
  chinese: 'zh',
  mandarin: 'zh',
  cantonese: 'yue',
  farsi: 'fa',
  persian: 'fa',
  burmese: 'my',
  myanmar: 'my',
  filipino: 'fil',
  tagalog: 'tl',
  jp: 'ja',
};

function canonicalCode(value: string): string | null {
  try {
    return Intl.getCanonicalLocales(value)[0] ?? null;
  } catch {
    return null;
  }
}

function languageDisplayName(code: string): string {
  try {
    return new Intl.DisplayNames(['en'], { type: 'language' }).of(code) ?? code;
  } catch {
    return code;
  }
}

function normalizeName(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[.?!,]/g, '')
    .replace(/\s+/g, ' ');
}

const LANGUAGE_NAME_TO_CODE = new Map<string, string>(
  ISO_639_1_CODES
    .map((code) => [normalizeName(languageDisplayName(code)), code] as const)
    .concat(Object.entries(LANGUAGE_ALIASES)),
);

export const TRANSLATOR_LANGUAGE_OPTIONS: TranslatorLanguage[] = ISO_639_1_CODES
  .map((code) => ({ code, name: languageDisplayName(code) }))
  .sort((a, b) => a.name.localeCompare(b.name));

export function normaliseTranslatorLanguage(value: string): TranslatorLanguage | null {
  const cleaned = normalizeName(value);
  if (!cleaned) return null;

  const aliased = LANGUAGE_NAME_TO_CODE.get(cleaned);
  const code = canonicalCode(aliased ?? cleaned);
  if (!code) return null;

  return {
    code,
    name: languageDisplayName(code),
  };
}

