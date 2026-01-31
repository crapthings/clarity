import { useEffect, useState, useCallback, useRef } from 'react'
import Icon from '@mdi/react'
import { mdiCamera, mdiFileDocumentOutline, mdiWeb, mdiNumeric } from '@mdi/js'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useTranslation } from '../i18n'

export default function Statistics () {
  const { t } = useTranslation()
  const [stats, setStats] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  
  // 使用 ref 存储最新的 t 函数，避免依赖项变化导致重复执行
  const tRef = useRef(t)
  useEffect(() => {
    tRef.current = t
  }, [t])

  const loadStatistics = useCallback(async () => {
    try {
      setError(null)
      console.log('Loading statistics...')
      const data = await invoke('get_today_statistics')
      console.log('Statistics data:', JSON.stringify(data, null, 2))
      if (data) {
        setStats(data)
      } else {
        setError(tRef.current('failedToLoadStatistics'))
      }
    } catch (err) {
      const errorMsg = err?.toString() || String(err)
      setError(errorMsg)
      console.error('Failed to load statistics:', err)
    } finally {
      setLoading(false)
    }
  }, []) // 空依赖数组，确保函数引用稳定

  useEffect(() => {
    let isMounted = true
    let unlistenFn = null
    let interval = null

    // 初始加载
    loadStatistics()

    // 监听统计更新事件
    const setupListener = async () => {
      try {
        const unlisten = await listen('statistics-updated', () => {
          console.log('Statistics updated event received, refreshing...')
          if (isMounted) {
            loadStatistics()
          }
        })
        unlistenFn = unlisten
      } catch (err) {
        console.error('Failed to setup statistics listener:', err)
      }
    }
    setupListener()

    // 每5秒刷新一次
    interval = setInterval(() => {
      if (isMounted) {
        loadStatistics()
      }
    }, 5000)

    return () => {
      isMounted = false
      if (interval) {
        clearInterval(interval)
      }
      if (unlistenFn) {
        try {
          unlistenFn()
        } catch (err) {
          console.error('Failed to unlisten:', err)
        }
      }
    }
    // loadStatistics 现在有稳定的引用（空依赖数组），所以这个 useEffect 只会在挂载时执行一次
  }, [loadStatistics])

  if (loading) {
    return (
      <div className='p-8'>
        <div className='text-gray-500'>{t('loadingStatistics')}</div>
      </div>
    )
  }

  if (error) {
    return (
      <div className='p-8'>
        <div className='text-red-600'>{t('failedToLoadStatistics')}: {error}</div>
        <button
          onClick={loadStatistics}
          className='mt-4 px-4 py-2 bg-gray-900 text-white rounded-md hover:bg-gray-800'
        >
          {t('retry')}
        </button>
      </div>
    )
  }

  if (!stats) {
    return (
      <div className='p-8'>
        <div className='text-gray-500'>{t('noDataAvailable')}</div>
        <button
          onClick={loadStatistics}
          className='mt-4 px-4 py-2 bg-gray-900 text-white rounded-md hover:bg-gray-800'
        >
          {t('refresh')}
        </button>
      </div>
    )
  }

  // 确保 apiStatistics 存在，如果不存在则使用默认值
  // 注意：后端使用 camelCase 序列化
  const apiStats = stats.apiStatistics || {
    totalRequests: 0,
    successfulRequests: 0,
    failedRequests: 0,
    totalPromptTokens: 0,
    totalCompletionTokens: 0,
    totalTokens: 0,
    avgDurationMs: null
  }

  const screenshotCount = stats.screenshotCount || 0
  const summaryCount = stats.summaryCount || 0

  const StatCard = ({ title, value, subtitle, icon }) => (
    <div className='bg-white border border-gray-200 rounded-lg p-4 shadow-sm hover:shadow-md transition-shadow'>
      <div className='flex items-center justify-between mb-1.5'>
        <h3 className='text-xs font-medium text-gray-600'>{title}</h3>
        <Icon path={icon} size={1.2} className='text-gray-400' />
      </div>
      <div className='text-2xl font-bold text-gray-900 mb-0.5'>{value}</div>
      {subtitle && (
        <div className='text-xs text-gray-500'>
          {typeof subtitle === 'string' ? subtitle : subtitle}
        </div>
      )}
    </div>
  )

  return (
    <div className='p-6 pb-20'>
      <div className='mb-6'>
        <h1 className='text-2xl font-semibold text-gray-900 mb-1'>{t('statisticsTitle')}</h1>
        <p className='text-xs text-gray-500'>{t('statisticsDescription')}</p>
      </div>

      <div className='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6'>
        <StatCard
          title={t('totalScreenshots')}
          value={screenshotCount.toLocaleString()}
          subtitle={t('todayCaptures')}
          icon={mdiCamera}
        />
        <StatCard
          title={t('totalSummaries')}
          value={summaryCount}
          subtitle={t('aiGeneratedSummaries')}
          icon={mdiFileDocumentOutline}
        />
        <StatCard
          title={t('apiRequests')}
          value={apiStats.totalRequests || 0}
          subtitle={`${t('success')}: ${apiStats.successfulRequests || 0} | ${t('failed')}: ${apiStats.failedRequests || 0}`}
          icon={mdiWeb}
        />
        <StatCard
          title={t('totalTokens')}
          value={(apiStats.totalTokens || 0).toLocaleString()}
          subtitle={
            <div className='space-y-0.5'>
              <div>{t('promptTokens')}: {(apiStats.totalPromptTokens || 0).toLocaleString()}</div>
              <div>{t('completionTokens')}: {(apiStats.totalCompletionTokens || 0).toLocaleString()}</div>
            </div>
          }
          icon={mdiNumeric}
        />
      </div>

      <div className='grid grid-cols-1 lg:grid-cols-2 gap-4'>
        <div className='bg-white border border-gray-200 rounded-lg p-4 shadow-sm'>
          <h2 className='text-base font-semibold text-gray-900 mb-3'>{t('apiPerformance')}</h2>
          <div className='space-y-3'>
            <div className='flex justify-between items-center'>
              <span className='text-gray-600'>{t('avgResponseTime')}</span>
              <span className='font-semibold text-gray-900'>
                {apiStats.avgDurationMs
                  ? `${(apiStats.avgDurationMs / 1000).toFixed(2)}s`
                  : 'N/A'}
              </span>
            </div>
            <div className='flex justify-between items-center'>
              <span className='text-gray-600'>{t('successRate')}</span>
              <span className='font-semibold text-gray-900'>
                {(apiStats.totalRequests || 0) > 0
                  ? `${(((apiStats.successfulRequests || 0) / apiStats.totalRequests) * 100).toFixed(1)}%`
                  : '0%'}
              </span>
            </div>
          </div>
        </div>

        <div className='bg-white border border-gray-200 rounded-lg p-4 shadow-sm'>
          <h2 className='text-base font-semibold text-gray-900 mb-3'>{t('tokenDistribution')}</h2>
          <div className='space-y-3'>
            <div>
              <div className='flex justify-between mb-1'>
                <span className='text-sm text-gray-600'>{t('promptTokens')}</span>
                <span className='text-sm font-medium text-gray-900'>
                  {(apiStats.totalPromptTokens || 0).toLocaleString()}
                </span>
              </div>
              <div className='w-full bg-gray-200 rounded-full h-2'>
                <div
                  className='bg-gray-600 h-2 rounded-full'
                  style={{
                    width: (apiStats.totalTokens || 0) > 0
                      ? `${((apiStats.totalPromptTokens || 0) / apiStats.totalTokens) * 100}%`
                      : '0%'
                  }}
                />
              </div>
            </div>
            <div>
              <div className='flex justify-between mb-1'>
                <span className='text-sm text-gray-600'>{t('completionTokens')}</span>
                <span className='text-sm font-medium text-gray-900'>
                  {(apiStats.totalCompletionTokens || 0).toLocaleString()}
                </span>
              </div>
              <div className='w-full bg-gray-200 rounded-full h-2'>
                <div
                  className='bg-gray-900 h-2 rounded-full'
                  style={{
                    width: (apiStats.totalTokens || 0) > 0
                      ? `${((apiStats.totalCompletionTokens || 0) / apiStats.totalTokens) * 100}%`
                      : '0%'
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
