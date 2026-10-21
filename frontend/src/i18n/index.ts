import zhCn from './zh-cn.json'
import zhTw from './zh-tw.json'
import enUs from './en-us.json'
import jaJp from './ja-jp.json'

export const languages = [
  { value: 'zh_cn', label: '简体中文' },
  { value: 'zh_tw', label: '繁體中文' },
  { value: 'en_us', label: 'English' },
  { value: 'ja_jp', label: '日本語' },
] as const

export type Locale = typeof languages[number]['value']
export type MessageKey = keyof typeof zhCn

const dictionaries: Record<Locale, Record<MessageKey, string>> = {
  zh_cn: zhCn, zh_tw: zhTw, en_us: enUs, ja_jp: jaJp,
}

export function hasLocale(value: unknown): value is Locale {
  return languages.some(language => language.value === value)
}

const saved = localStorage.getItem('locale')
const browser = navigator.language.toLowerCase().replace('-', '_')
export const locale: Locale = hasLocale(saved) ? saved : hasLocale(browser) ? browser : 'zh_cn'
export const dateLocale = locale.replace('_', '-')

export function setLocale(value: Locale) {
  localStorage.setItem('locale', value)
  location.reload()
}

export function t(key: MessageKey, vars: Record<string, string | number> = {}): string {
  return dictionaries[locale][key].replace(/{{\s*(\w+)\s*}}/g, (_, name: string) => String(vars[name]))
}

const errorKeys: Record<string, MessageKey> = {
  'unauthorized': 'pdf.expired',
  'account not activated': 'error.inactive',
  'not a game admin': 'error.notAdmin',
  'game not found': 'game.notFound',
  'writeup not found': 'pdf.notFound',
  'no writeup submitted for this game': 'reader.noSubmission',
  'this game does not accept writeup submissions': 'error.closed',
  'competition has not started yet': 'submit.notStarted',
  'competition has ended': 'submit.ended',
  'you are not registered for this competition': 'submit.notRegistered',
  'duplicate writeup: this file has already been submitted in this game': 'error.duplicate',
  'no file field in request': 'error.missingFile',
  'payload too large': 'upload.tooLarge',
  'file too small': 'error.tooSmall',
  'ZIP file is not a valid docx (missing [Content_Types].xml)': 'error.invalidDocx',
  'unsupported file type: only PDF and docx are accepted': 'upload.unsupported',
  'unsupported file type for this game': 'error.formatDisabled',
  'unsupported file format': 'error.unsupportedFormat',
  'at least one file format must be allowed': 'error.noFormatSelected',
  'internal server error': 'error.internal',
  '验证码无效或已过期': 'login.expired',
  '验证码验证失败': 'error.codeFailed',
}

export function translateError(message: string | undefined, fallback: MessageKey): string {
  const key = errorKeys[message ?? '']
  return key ? t(key) : message || t(fallback)
}
