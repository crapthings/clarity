import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../store'
import { useTranslation } from '../i18n'

export default function Settings () {
  const { t } = useTranslation()
  const language = useAppStore((state) => state.language || 'en')
  const setLanguage = useAppStore((state) => state.setLanguage)
  const [apiKey, setApiKey] = useState('')
  const [summaryInterval, setSummaryInterval] = useState(45)
  const [aiModel, setAiModel] = useState('gemini-3-flash-preview')
  const [aiPrompt, setAiPrompt] = useState('')
  const [videoResolution, setVideoResolution] = useState('low')
  const [saving, setSaving] = useState(false)
  const [savingInterval, setSavingInterval] = useState(false)
  const [savingModel, setSavingModel] = useState(false)
  const [savingPrompt, setSavingPrompt] = useState(false)
  const [savingResolution, setSavingResolution] = useState(false)
  const [apiKeyMessage, setApiKeyMessage] = useState(null)
  const [intervalMessage, setIntervalMessage] = useState(null)
  const [modelMessage, setModelMessage] = useState(null)
  const [promptMessage, setPromptMessage] = useState(null)
  const [resolutionMessage, setResolutionMessage] = useState(null)

  // 默认提示词（按语言）
  const defaultPrompts = {
    zh: '分析这段屏幕活动视频，提供简洁的活动摘要。重点关注：1) 主要使用的应用/网站；2) 活动类型（工作/娱乐/学习等）；3) 是否有分心或低效行为。用中文回答，控制在100字以内。',
    en: 'Analyze this screen activity video and provide a concise activity summary. Focus on: 1) Main apps/websites used; 2) Activity type (work/entertainment/learning, etc.); 3) Any distractions or inefficient behaviors. Respond in English, keep it under 100 words.'
  }

  useEffect(() => {
    loadApiKey()
    loadSummaryInterval()
    loadAiModel()
    loadAiPrompt()
    loadLanguage()
    loadVideoResolution()
  }, [])

  // 当语言切换时，重新加载对应语言的提示词
  // 注意：语言切换的保存已经在 select 的 onChange 中处理
  useEffect(() => {
    loadAiPrompt()
  }, [language])

  const loadLanguage = async () => {
    try {
      const savedLanguage = await invoke('get_language')
      if (savedLanguage && (savedLanguage === 'en' || savedLanguage === 'zh')) {
        // 如果后端有语言设置且与前端不同，同步到前端
        if (savedLanguage !== language) {
          setLanguage(savedLanguage)
        }
      } else {
        // 如果后端没有语言设置，将前端的语言设置保存到后端
        await invoke('set_language', { language })
      }
    } catch (err) {
      console.error('Failed to load language:', err)
      // 如果加载失败，尝试将前端语言保存到后端
      try {
        await invoke('set_language', { language })
      } catch (saveErr) {
        console.error('Failed to save language to backend:', saveErr)
      }
    }
  }

  const loadApiKey = async () => {
    try {
      const key = await invoke('get_gemini_api_key')
      setApiKey(key || '')
    } catch (err) {
      console.error('Failed to load API key:', err)
    }
  }

  const loadSummaryInterval = async () => {
    try {
      const interval = await invoke('get_summary_interval')
      setSummaryInterval(interval || 45)
    } catch (err) {
      console.error('Failed to load summary interval:', err)
    }
  }

  const loadAiModel = async () => {
    try {
      const model = await invoke('get_ai_model')
      setAiModel(model || 'gemini-3-flash-preview')
    } catch (err) {
      console.error('Failed to load AI model:', err)
    }
  }

  const loadAiPrompt = async () => {
    try {
      const prompt = await invoke('get_ai_prompt', { language })
      setAiPrompt(prompt || defaultPrompts[language] || defaultPrompts.zh)
    } catch (err) {
      console.error('Failed to load AI prompt:', err)
      setAiPrompt(defaultPrompts[language] || defaultPrompts.zh)
    }
  }

  const loadVideoResolution = async () => {
    try {
      const resolution = await invoke('get_video_resolution')
      setVideoResolution(resolution || 'low')
    } catch (err) {
      console.error('Failed to load video resolution:', err)
    }
  }

  const saveVideoResolution = async () => {
    setSavingResolution(true)
    setResolutionMessage(null)
    try {
      await invoke('set_video_resolution', { resolution: videoResolution })
      setResolutionMessage({ type: 'success', text: t('resolutionSavedSuccessfully') })
      setTimeout(() => setResolutionMessage(null), 3000)
    } catch (err) {
      console.error('Failed to save video resolution:', err)
      setResolutionMessage({ type: 'error', text: err.toString() })
    } finally {
      setSavingResolution(false)
    }
  }

  const saveApiKey = async () => {
    setSaving(true)
    setApiKeyMessage(null)
    try {
      await invoke('set_gemini_api_key', { apiKey })
      setApiKeyMessage({ type: 'success', text: t('apiKeySavedSuccessfully') })
      setTimeout(() => setApiKeyMessage(null), 3000)
    } catch (err) {
      console.error('Failed to save API key:', err)
      setApiKeyMessage({ type: 'error', text: err.toString() })
    } finally {
      setSaving(false)
    }
  }

  const saveSummaryInterval = async () => {
    setSavingInterval(true)
    setIntervalMessage(null)
    try {
      await invoke('set_summary_interval', { intervalSeconds: summaryInterval })
      setIntervalMessage({ type: 'success', text: t('intervalSavedSuccessfully') })
      setTimeout(() => setIntervalMessage(null), 3000)
    } catch (err) {
      console.error('Failed to save summary interval:', err)
      setIntervalMessage({ type: 'error', text: err.toString() })
    } finally {
      setSavingInterval(false)
    }
  }

  const saveAiModel = async () => {
    setSavingModel(true)
    setModelMessage(null)
    try {
      await invoke('set_ai_model', { model: aiModel })
      setModelMessage({ type: 'success', text: t('modelSavedSuccessfully') })
      setTimeout(() => setModelMessage(null), 3000)
    } catch (err) {
      console.error('Failed to save AI model:', err)
      setModelMessage({ type: 'error', text: err.toString() })
    } finally {
      setSavingModel(false)
    }
  }

  const saveAiPrompt = async () => {
    setSavingPrompt(true)
    setPromptMessage(null)
    try {
      await invoke('set_ai_prompt', { prompt: aiPrompt, language })
      setPromptMessage({ type: 'success', text: t('promptSavedSuccessfully') })
      setTimeout(() => setPromptMessage(null), 3000)
    } catch (err) {
      console.error('Failed to save AI prompt:', err)
      setPromptMessage({ type: 'error', text: err.toString() })
    } finally {
      setSavingPrompt(false)
    }
  }

  const resetPrompt = async () => {
    setSavingPrompt(true)
    setPromptMessage(null)
    try {
      const resetPrompt = await invoke('reset_ai_prompt', { language })
      setAiPrompt(resetPrompt)
      setPromptMessage({ type: 'success', text: t('promptResetToDefault') })
      setTimeout(() => setPromptMessage(null), 3000)
    } catch (err) {
      console.error('Failed to reset prompt:', err)
      setPromptMessage({ type: 'error', text: err.toString() })
    } finally {
      setSavingPrompt(false)
    }
  }

  return (
    <div className='p-6 bg-white pb-20'>
      <div className='mb-6'>
        <h2 className='text-2xl font-semibold text-gray-900 mb-1'>{t('settingsTitle')}</h2>
        <p className='text-xs text-gray-500'>{t('settingsDescription')}</p>
      </div>

      <div className='max-w-2xl space-y-4'>
        {/* Language */}
        <div className='bg-white border border-gray-200 rounded-lg p-4'>
          <h3 className='text-base font-semibold text-gray-900 mb-3'>{t('language')}</h3>
          <p className='text-sm text-gray-600 mb-4'>
            {t('languageDescription')}
          </p>
          <div className='space-y-4'>
            <div>
              <label
                htmlFor='language'
                className='block text-sm font-medium text-gray-700 mb-2'
              >
                {t('language')}
              </label>
              <select
                id='language'
                value={language}
                onChange={async (e) => {
                  const newLanguage = e.target.value
                  setLanguage(newLanguage)
                  // 保存语言设置到后端
                  try {
                    await invoke('set_language', { language: newLanguage })
                  } catch (err) {
                    console.error('Failed to save language to backend:', err)
                  }
                  // 重新加载对应语言的提示词
                  await loadAiPrompt()
                }}
                className='w-full px-4 py-2.5 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-gray-900 focus:border-gray-900 transition-all'
              >
                <option value='en'>English</option>
                <option value='zh'>中文</option>
              </select>
            </div>
          </div>
        </div>
        {/* Google Gemini API Key */}
        <div className='bg-white border border-gray-200 rounded-lg p-4'>
          <h3 className='text-base font-semibold text-gray-900 mb-3'>{t('openRouterApiKey')}</h3>
          <p className='text-sm text-gray-600 mb-4'>
            {t('openRouterApiKeyDescription')}{' '}
            <a
              href='https://aistudio.google.com/app/apikey'
              target='_blank'
              rel='noopener noreferrer'
              className='text-gray-900 hover:text-gray-700 underline'
            >
              aistudio.google.com/app/apikey
            </a>
          </p>

          {apiKeyMessage && (
            <div
              className={`mb-4 p-3 rounded-lg border ${
                apiKeyMessage.type === 'success'
                  ? 'bg-gray-50 text-gray-700 border-gray-200'
                  : 'bg-gray-50 text-gray-700 border-gray-200'
              }`}
            >
              {apiKeyMessage.text}
            </div>
          )}

          <div className='space-y-4'>
            <div>
              <label
                htmlFor='api-key'
                className='block text-sm font-medium text-gray-700 mb-2'
              >
                {t('apiKey')}
              </label>
              <input
                id='api-key'
                type='password'
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder='AIza...'
                className='w-full px-4 py-2.5 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-gray-900 focus:border-gray-900 transition-all'
              />
            </div>

            <button
              onClick={saveApiKey}
              disabled={saving}
              className='px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium'
            >
              {saving ? t('saving') : t('saveApiKey')}
            </button>
          </div>
        </div>

        {/* AI Summary Interval */}
        <div className='bg-white border border-gray-200 rounded-lg p-4'>
          <h3 className='text-base font-semibold text-gray-900 mb-3'>{t('aiSummaryInterval')}</h3>
          <p className='text-sm text-gray-600 mb-4'>
            {t('aiSummaryIntervalDescription')}
          </p>

          {intervalMessage && (
            <div
              className={`mb-4 p-3 rounded-lg border ${
                intervalMessage.type === 'success'
                  ? 'bg-gray-50 text-gray-700 border-gray-200'
                  : 'bg-gray-50 text-gray-700 border-gray-200'
              }`}
            >
              {intervalMessage.text}
            </div>
          )}

          <div className='space-y-4'>
            <div>
              <label
                htmlFor='summary-interval'
                className='block text-sm font-medium text-gray-700 mb-2'
              >
                {t('intervalSeconds')}
              </label>
              <input
                id='summary-interval'
                type='number'
                min='10'
                max='3600'
                value={summaryInterval}
                onChange={(e) => setSummaryInterval(parseInt(e.target.value) || 45)}
                className='w-full px-4 py-2.5 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-gray-900 focus:border-gray-900 transition-all'
              />
              <p className='mt-2 text-xs text-gray-500'>
                {t('minMaxInterval')}
              </p>
            </div>

            <button
              onClick={saveSummaryInterval}
              disabled={savingInterval}
              className='px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium'
            >
              {savingInterval ? t('saving') : t('saveInterval')}
            </button>
          </div>
        </div>

        {/* AI Model */}
        <div className='bg-white border border-gray-200 rounded-lg p-4'>
          <h3 className='text-base font-semibold text-gray-900 mb-3'>{t('aiModel')}</h3>
          <p className='text-sm text-gray-600 mb-4'>
            {t('aiModelDescription')}
          </p>

          {modelMessage && (
            <div
              className={`mb-4 p-3 rounded-lg border ${
                modelMessage.type === 'success'
                  ? 'bg-gray-50 text-gray-700 border-gray-200'
                  : 'bg-gray-50 text-gray-700 border-gray-200'
              }`}
            >
              {modelMessage.text}
            </div>
          )}

          <div className='space-y-4'>
            <div>
              <label
                htmlFor='ai-model'
                className='block text-sm font-medium text-gray-700 mb-2'
              >
                {t('modelId')}
              </label>
              <input
                id='ai-model'
                type='text'
                value={aiModel}
                onChange={(e) => setAiModel(e.target.value)}
                placeholder='gemini-3-flash-preview'
                className='w-full px-4 py-2.5 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-gray-900 focus:border-gray-900 transition-all font-mono text-sm'
              />
              <p className='mt-2 text-xs text-gray-500'>
                {t('findModelsAt')}{' '}
                <a
                  href='https://ai.google.dev/models'
                  target='_blank'
                  rel='noopener noreferrer'
                  className='text-gray-900 hover:text-gray-700 underline'
                >
                  ai.google.dev/models
                </a>
                . {t('makeSureModelSupportsVideo')}
              </p>
            </div>

            <button
              onClick={saveAiModel}
              disabled={savingModel}
              className='px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium'
            >
              {savingModel ? t('saving') : t('saveModel')}
            </button>
          </div>
        </div>

        {/* AI Prompt */}
        <div className='bg-white border border-gray-200 rounded-lg p-4'>
          <h3 className='text-base font-semibold text-gray-900 mb-3'>{t('aiPrompt')}</h3>
          <p className='text-sm text-gray-600 mb-4'>
            {t('aiPromptDescription')}
            <span className='block mt-2 text-xs text-gray-500'>
              {language === 'zh' 
                ? '提示词会根据当前语言自动切换。当前语言：中文' 
                : 'Prompt will automatically switch based on current language. Current language: English'}
            </span>
          </p>

          {promptMessage && (
            <div
              className={`mb-4 p-3 rounded-lg border ${
                promptMessage.type === 'success'
                  ? 'bg-gray-50 text-gray-700 border-gray-200'
                  : 'bg-gray-50 text-gray-700 border-gray-200'
              }`}
            >
              {promptMessage.text}
            </div>
          )}

          <div className='space-y-4'>
            <div>
              <label
                htmlFor='ai-prompt'
                className='block text-sm font-medium text-gray-700 mb-2'
              >
                {t('prompt')}
              </label>
              <textarea
                id='ai-prompt'
                value={aiPrompt}
                onChange={(e) => setAiPrompt(e.target.value)}
                rows={6}
                placeholder={language === 'zh' ? '输入您的自定义提示词...' : 'Enter your custom prompt...'}
                className='w-full px-4 py-2.5 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-gray-900 focus:border-gray-900 transition-all font-mono text-sm'
              />
            </div>

            <div className='flex gap-2'>
              <button
                onClick={saveAiPrompt}
                disabled={savingPrompt}
                className='px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium'
              >
                {savingPrompt ? t('saving') : t('savePrompt')}
              </button>
              <button
                onClick={resetPrompt}
                disabled={savingPrompt}
                className='px-4 py-2 bg-gray-200 text-gray-900 text-sm rounded-lg hover:bg-gray-300 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium'
              >
                {t('resetToDefault')}
              </button>
            </div>
          </div>
        </div>

        {/* Video Resolution */}
        <div className='bg-white border border-gray-200 rounded-lg p-4'>
          <h3 className='text-base font-semibold text-gray-900 mb-3'>{t('videoResolution')}</h3>
          <p className='text-sm text-gray-600 mb-4'>
            {t('videoResolutionDescription')}
          </p>

          {resolutionMessage && (
            <div
              className={`mb-4 p-3 rounded-lg border ${
                resolutionMessage.type === 'success'
                  ? 'bg-gray-50 text-gray-700 border-gray-200'
                  : 'bg-gray-50 text-gray-700 border-gray-200'
              }`}
            >
              {resolutionMessage.text}
            </div>
          )}

          <div className='space-y-4'>
            <div>
              <label
                htmlFor='video-resolution'
                className='block text-sm font-medium text-gray-700 mb-2'
              >
                {t('resolution')}
              </label>
              <select
                id='video-resolution'
                value={videoResolution}
                onChange={(e) => setVideoResolution(e.target.value)}
                className='w-full px-4 py-2.5 border border-gray-300 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-gray-900 focus:border-gray-900 transition-all'
              >
                <option value='low'>{t('lowResolution')} ({t('lowResolutionDesc')})</option>
                <option value='default'>{t('defaultResolution')} ({t('defaultResolutionDesc')})</option>
              </select>
              <p className='mt-2 text-xs text-gray-500'>
                {t('resolutionNote')}
              </p>
            </div>

            <button
              onClick={saveVideoResolution}
              disabled={savingResolution}
              className='px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium'
            >
              {savingResolution ? t('saving') : t('saveResolution')}
            </button>
          </div>
        </div>

        {/* Storage Info */}
        <div className='bg-white border border-gray-200 rounded-lg p-4'>
          <h3 className='text-base font-semibold text-gray-900 mb-3'>{t('storage')}</h3>
          <div className='space-y-2 text-sm'>
            <div className='flex justify-between items-center'>
              <span className='text-gray-600'>{t('storagePath')}</span>
              <code className='text-xs bg-gray-100 px-2 py-1 rounded text-gray-800'>
                ~/Library/Application Support/clarity/recordings/
              </code>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
