import { useAppStore } from '../store'
import { translations } from './locales'

export const useTranslation = () => {
  // 订阅语言变化，确保组件在语言改变时重新渲染
  const language = useAppStore((state) => state.language || 'en')

  const t = (key) => {
    return translations[language]?.[key] || translations.en[key] || key
  }

  return { t, language }
}
