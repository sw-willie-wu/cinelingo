export type SourceLang = string // 'auto' | whisper ISO | 'zh-Hant' | 'zh-Hans'

export interface LangDef { value: string; label: string; whisper: string | null; prompt?: string }

const PROMPT_TRAD = '以下是繁體中文的內容。'
const PROMPT_SIMP = '以下是简体中文的内容。'

// 常用在前（auto, 繁中, 簡中, 日, 英, 韓），其餘為 whisper 支援語言（ISO）。
export const LANGS: LangDef[] = [
  { value: 'auto', label: '自動偵測', whisper: null },
  { value: 'zh-Hant', label: '繁體中文', whisper: 'zh', prompt: PROMPT_TRAD },
  { value: 'zh-Hans', label: '简体中文', whisper: 'zh', prompt: PROMPT_SIMP },
  { value: 'ja', label: '日語', whisper: 'ja' },
  { value: 'en', label: '英語', whisper: 'en' },
  { value: 'ko', label: '韓語', whisper: 'ko' },
  { value: 'es', label: '西班牙語', whisper: 'es' },
  { value: 'fr', label: '法語', whisper: 'fr' },
  { value: 'de', label: '德語', whisper: 'de' },
  { value: 'ru', label: '俄語', whisper: 'ru' },
  { value: 'pt', label: '葡萄牙語', whisper: 'pt' },
  { value: 'it', label: '義大利語', whisper: 'it' },
  { value: 'nl', label: '荷蘭語', whisper: 'nl' },
  { value: 'ar', label: '阿拉伯語', whisper: 'ar' },
  { value: 'hi', label: '印地語', whisper: 'hi' },
  { value: 'tr', label: '土耳其語', whisper: 'tr' },
  { value: 'vi', label: '越南語', whisper: 'vi' },
  { value: 'th', label: '泰語', whisper: 'th' },
  { value: 'id', label: '印尼語', whisper: 'id' },
  { value: 'pl', label: '波蘭語', whisper: 'pl' },
  { value: 'uk', label: '烏克蘭語', whisper: 'uk' },
  // 其餘 whisper 支援語言（ISO 碼，label 用英文/通用名；可日後在地化）：
  ...([
    ['ca','Catalan'],['sv','Swedish'],['fi','Finnish'],['he','Hebrew'],['el','Greek'],['ms','Malay'],
    ['cs','Czech'],['ro','Romanian'],['da','Danish'],['hu','Hungarian'],['ta','Tamil'],['no','Norwegian'],
    ['ur','Urdu'],['hr','Croatian'],['bg','Bulgarian'],['lt','Lithuanian'],['la','Latin'],['mi','Maori'],
    ['ml','Malayalam'],['cy','Welsh'],['sk','Slovak'],['te','Telugu'],['fa','Persian'],['lv','Latvian'],
    ['bn','Bengali'],['sr','Serbian'],['az','Azerbaijani'],['sl','Slovenian'],['kn','Kannada'],['et','Estonian'],
    ['mk','Macedonian'],['br','Breton'],['eu','Basque'],['is','Icelandic'],['hy','Armenian'],['ne','Nepali'],
    ['mn','Mongolian'],['bs','Bosnian'],['kk','Kazakh'],['sq','Albanian'],['sw','Swahili'],['gl','Galician'],
    ['mr','Marathi'],['pa','Punjabi'],['si','Sinhala'],['km','Khmer'],['sn','Shona'],['yo','Yoruba'],
    ['so','Somali'],['af','Afrikaans'],['oc','Occitan'],['ka','Georgian'],['be','Belarusian'],['tg','Tajik'],
    ['sd','Sindhi'],['gu','Gujarati'],['am','Amharic'],['yi','Yiddish'],['lo','Lao'],['uz','Uzbek'],
    ['fo','Faroese'],['ht','Haitian Creole'],['ps','Pashto'],['tk','Turkmen'],['nn','Nynorsk'],['mt','Maltese'],
    ['sa','Sanskrit'],['lb','Luxembourgish'],['my','Myanmar'],['bo','Tibetan'],['tl','Tagalog'],['mg','Malagasy'],
    ['as','Assamese'],['tt','Tatar'],['haw','Hawaiian'],['ln','Lingala'],['ha','Hausa'],['ba','Bashkir'],
    ['jw','Javanese'],['su','Sundanese'],['yue','Cantonese'],
  ] as [string, string][]).map(([value, label]) => ({ value, label, whisper: value })),
]

const BY_VALUE = new Map(LANGS.map((l) => [l.value, l]))

/** sourceLang → 給 whisper 的 language(ISO) 與 initial_prompt。 */
export function langToWhisper(sourceLang: string): { lang: string | null; prompt: string | null } {
  const d = BY_VALUE.get(sourceLang)
  if (!d || d.value === 'auto') return { lang: null, prompt: null }
  return { lang: d.whisper, prompt: d.prompt ?? null }
}
