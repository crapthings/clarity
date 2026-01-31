import { useState, useEffect, useMemo } from 'react'
import { invoke } from '@tauri-apps/api/core'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { useTranslation } from '../i18n'
import { useAppStore } from '../store'

export default function Trace () {
  const { t } = useTranslation()
  const language = useAppStore((state) => state.language || 'en')
  const [summaries, setSummaries] = useState([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)
  const [currentDate, setCurrentDate] = useState(new Date())

  // 获取指定日期的开始和结束时间
  const getDateRange = (date) => {
    const start = new Date(date)
    start.setHours(0, 0, 0, 0)
    const end = new Date(date)
    end.setHours(23, 59, 59, 999)
    return {
      start: start.toISOString(),
      end: end.toISOString()
    }
  }

  const loadSummaries = async (date) => {
    setLoading(true)
    setError(null)
    try {
      const { start, end } = getDateRange(date)
      const data = await invoke('get_summaries', {
        startTime: start,
        endTime: end,
        limit: 200
      })
      setSummaries(data || [])
    } catch (err) {
      console.error('Failed to load summaries:', err)
      setError(err.toString())
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadSummaries(currentDate)
  }, [currentDate])

  const goToPreviousDay = () => {
    const prev = new Date(currentDate)
    prev.setDate(prev.getDate() - 1)
    setCurrentDate(prev)
  }

  const goToNextDay = () => {
    const next = new Date(currentDate)
    next.setDate(next.getDate() + 1)
    setCurrentDate(next)
  }

  const goToToday = () => {
    setCurrentDate(new Date())
  }

  const formatTime = (dateString) => {
    const date = new Date(dateString)
    return date.toLocaleTimeString(language === 'zh' ? 'zh-CN' : 'en-US', {
      hour: '2-digit',
      minute: '2-digit'
    })
  }

  const formatDisplayDate = (date) => {
    const today = new Date()
    const yesterday = new Date(today)
    yesterday.setDate(yesterday.getDate() - 1)

    if (date.toDateString() === today.toDateString()) {
      return t('today')
    } else if (date.toDateString() === yesterday.toDateString()) {
      return t('yesterday')
    } else {
      return date.toLocaleDateString(language === 'zh' ? 'zh-CN' : 'en-US', {
        month: 'short',
        day: 'numeric',
        year: date.getFullYear() !== today.getFullYear() ? 'numeric' : undefined
      })
    }
  }

  const formatCurrentTime = () => {
    const now = new Date()
    return now.toLocaleTimeString(language === 'zh' ? 'zh-CN' : 'en-US', {
      hour: '2-digit',
      minute: '2-digit'
    })
  }

  const formatCurrentDate = () => {
    const now = new Date()
    return now.toLocaleDateString(language === 'zh' ? 'zh-CN' : 'en-US', {
      weekday: 'short',
      month: 'short',
      day: 'numeric'
    })
  }

  const calculateHeatmap = useMemo(() => {
    if (summaries.length === 0) {
      return { entertainment: 0, productivity: 0 }
    }

    const entertainmentKeywords = ['video', 'game', 'entertainment', 'watch', 'play', 'movie', 'music', 'social', 'browse', 'fun', 'relax', '视频', '游戏', '娱乐', '观看', '播放', '电影', '音乐', '社交', '浏览', '放松']
    const productivityKeywords = ['work', 'code', 'write', 'read', 'study', 'learn', 'meeting', 'email', 'document', 'project', 'task', '工作', '代码', '写作', '阅读', '学习', '会议', '邮件', '文档', '项目', '任务']

    let entertainmentScore = 0
    let productivityScore = 0

    summaries.forEach(summary => {
      const content = summary.content.toLowerCase()
      const entertainmentMatches = entertainmentKeywords.filter(keyword => content.includes(keyword.toLowerCase())).length
      const productivityMatches = productivityKeywords.filter(keyword => content.includes(keyword.toLowerCase())).length

      entertainmentScore += entertainmentMatches
      productivityScore += productivityMatches
    })

    const total = entertainmentScore + productivityScore
    if (total === 0) {
      return { entertainment: 0, productivity: 0 }
    }

    return {
      entertainment: Math.round((entertainmentScore / total) * 100),
      productivity: Math.round((productivityScore / total) * 100)
    }
  }, [summaries])

  // 标签颜色映射（使用 tailwind 300 色系）
  const tagColorMap = {
    'browser': 'bg-blue-300',
    'code': 'bg-purple-300',
    'write': 'bg-green-300',
    'read': 'bg-cyan-300',
    'meeting': 'bg-orange-300',
    'email': 'bg-yellow-300',
    'video': 'bg-red-300',
    'music': 'bg-pink-300',
    'chat': 'bg-indigo-300',
    'design': 'bg-teal-300',
    '浏览器': 'bg-blue-300',
    '代码': 'bg-purple-300',
    '写作': 'bg-green-300',
    '阅读': 'bg-cyan-300',
    '会议': 'bg-orange-300',
    '邮件': 'bg-yellow-300',
    '视频': 'bg-red-300',
    '音乐': 'bg-pink-300',
    '聊天': 'bg-indigo-300',
    '设计': 'bg-teal-300'
  }

  const activityTags = useMemo(() => {
    if (summaries.length === 0) return []

    // 从所有当天的摘要中提取唯一标签
    const tagSet = new Set()

    summaries.forEach(summary => {
      const content = summary.content.toLowerCase()
      const commonTags = [
        'browser', 'code', 'write', 'read', 'meeting', 'email', 'video', 'music', 'chat', 'design',
        '浏览器', '代码', '写作', '阅读', '会议', '邮件', '视频', '音乐', '聊天', '设计'
      ]

      commonTags.forEach(tag => {
        if (content.includes(tag.toLowerCase())) {
          tagSet.add(tag)
        }
      })
    })

    // 返回唯一标签数组
    return Array.from(tagSet)
  }, [summaries])

  const totalHours = useMemo(() => {
    if (summaries.length === 0) return 0
    const first = new Date(summaries[summaries.length - 1].startTime)
    const last = new Date(summaries[0].endTime)
    const diffMs = last - first
    return (diffMs / (1000 * 60 * 60)).toFixed(1)
  }, [summaries])

  const activeHours = useMemo(() => {
    const hours = new Set()
    summaries.forEach(summary => {
      const start = new Date(summary.startTime)
      const end = new Date(summary.endTime)
      const startHour = start.getHours()
      const endHour = end.getHours()
      for (let hour = startHour; hour <= endHour; hour++) {
        hours.add(hour)
      }
    })
    return hours.size
  }, [summaries])

  const focusScore = useMemo(() => {
    return calculateHeatmap.productivity
  }, [calculateHeatmap])

  const sortedSummaries = useMemo(() => {
    return [...summaries].sort((a, b) => {
      return new Date(b.startTime) - new Date(a.startTime)
    })
  }, [summaries])

  return (
    <div className='flex flex-col h-full bg-white'>
      {/* Header: 左侧导航 + 右侧时间日期 */}
      <header className='flex items-center justify-between px-8 py-3 bg-white border-b border-gray-100'>
        <div className='flex items-center gap-1.5'>
          <button
            onClick={goToPreviousDay}
            className='w-7 h-7 flex items-center justify-center rounded hover:bg-gray-50 transition-all'
            title='Previous day'
          >
            <svg className='w-3.5 h-3.5 text-gray-400' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
              <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={3} d='M15 19l-7-7 7-7' />
            </svg>
          </button>
          <button
            onClick={goToToday}
            className='px-3 py-1 text-xs font-semibold text-gray-900 hover:bg-gray-50 rounded transition-all'
          >
            {formatDisplayDate(currentDate)}
          </button>
          <button
            onClick={goToNextDay}
            className='w-7 h-7 flex items-center justify-center rounded hover:bg-gray-50 transition-all disabled:opacity-20 disabled:cursor-not-allowed disabled:hover:bg-transparent'
            disabled={currentDate.toDateString() === new Date().toDateString()}
            title='Next day'
          >
            <svg className='w-3.5 h-3.5 text-gray-400' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
              <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={3} d='M9 5l7 7-7 7' />
            </svg>
          </button>
        </div>
        <div className='flex items-center gap-2.5'>
          <div className='text-base font-bold text-gray-900 tracking-tight tabular-nums'>{formatCurrentTime()}</div>
          <div className='w-px h-3.5 bg-gray-200' />
          <div className='text-xs text-gray-400 font-medium'>{formatCurrentDate()}</div>
        </div>
      </header>

      {/* Sub Header: 热度表 + 行为标签 */}
      <div className='px-8 py-4 bg-white border-b border-gray-100'>
        {summaries.length === 0 && !loading
          ? (
              <div className='text-center py-2'>
                <span className='text-sm font-medium text-gray-600'>{t('whatAWonderfulDay')}</span>
              </div>
            )
          : (
              <>
                {/* 热度表 - 灰度配色 */}
                <div className='mb-3'>
                  <div className='flex items-center justify-between mb-2'>
                    <div className='flex items-center gap-1.5'>
                      <div className='w-1.5 h-1.5 rounded-full bg-gray-300' />
                      <span className='text-xs font-medium text-gray-500 uppercase tracking-wider'>{t('entertainment')}</span>
                      <span className='text-xs text-gray-400 font-semibold tabular-nums'>{calculateHeatmap.entertainment}%</span>
                    </div>
                    <div className='flex items-center gap-1.5'>
                      <span className='text-xs text-gray-400 font-semibold tabular-nums'>{calculateHeatmap.productivity}%</span>
                      <span className='text-xs font-medium text-gray-500 uppercase tracking-wider'>{t('productivity')}</span>
                      <div className='w-1.5 h-1.5 rounded-full bg-gray-900' />
                    </div>
                  </div>
                  <div className='relative h-1 bg-gray-100 rounded-full overflow-hidden'>
                    <div
                      className='absolute left-0 top-0 h-full bg-gray-300 transition-all duration-500'
                      style={{ width: `${calculateHeatmap.entertainment}%` }}
                    />
                    <div
                      className='absolute right-0 top-0 h-full bg-gray-900 transition-all duration-500'
                      style={{ width: `${calculateHeatmap.productivity}%` }}
                    />
                  </div>
                </div>

                {/* 行为标签 - 横向可滚动 */}
                {activityTags.length > 0 && (
                    <div className='flex items-center gap-2.5'>
                      <span className='text-xs font-semibold text-gray-400 uppercase tracking-wider whitespace-nowrap flex-shrink-0'>{t('activityTags')}</span>
                      <div className='flex gap-1.5 overflow-x-auto scrollbar-hide flex-1'>
                        {activityTags.map((tag, index) => {
                          const bgColor = tagColorMap[tag] || 'bg-gray-300'
                          return (
                            <span
                              key={index}
                              className={`px-2.5 py-1 ${bgColor} rounded-md text-xs font-medium text-gray-800 whitespace-nowrap flex-shrink-0`}
                            >
                              {tag}
                            </span>
                          )
                        })}
                      </div>
                    </div>
                  )}
              </>
            )}
      </div>

      {/* Main Content: 时间轴列表 */}
      <main className='flex-1 overflow-y-auto px-8 py-6'>
        {error && (
          <div className='mb-6 p-4 bg-gray-50 border border-gray-200 rounded text-sm text-gray-600'>
            {t('failedToLoadSummaries')}: {error}
          </div>
        )}

        {loading && summaries.length === 0 && (
          <div className='text-center py-16 text-sm text-gray-400'>{t('loadingSummaries')}</div>
        )}

        {!loading && summaries.length === 0 && (
          <div className='text-center py-16 text-sm text-gray-400'>
            {t('noSummaries')}
          </div>
        )}

        {!loading && sortedSummaries.length > 0 && (
          <div className='max-w-4xl mx-auto'>
            <div className='relative'>
              {/* 时间轴线 */}
              <div className='absolute left-0 top-2 bottom-0 w-px bg-gray-200' />

              <div className='space-y-0'>
                {sortedSummaries.map((summary, index) => {
                  const startTime = formatTime(summary.startTime)
                  const isFirst = index === 0

                  return (
                    <div key={summary.id} className='relative pl-8 pb-6'>
                      {/* 时间标记和时间标签容器 */}
                      <div className='absolute left-0 top-2 -translate-x-1/2'>
                        <div className='relative flex items-center'>
                          {/* 圆点 */}
                          <div className='relative'>
                            <div className='w-2.5 h-2.5 rounded-full bg-gray-900 ring-2 ring-white shadow-sm z-10' />
                            {isFirst && (
                              <div className='absolute top-0 left-0 w-2.5 h-2.5 rounded-full bg-gray-900 animate-ping opacity-75' />
                            )}
                          </div>
                          
                          {/* 时间标签 - 在圆点左侧 */}
                          <div className='absolute right-full mr-3 flex items-center justify-center'>
                            <div className='text-xs font-semibold text-gray-900 tabular-nums whitespace-nowrap'>
                              {startTime}
                            </div>
                          </div>
                        </div>
                      </div>

                      {/* 内容卡片 - 扁平设计，无边框 */}
                      <div className='bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors'>
                        <div className='p-5'>
                          <div className='prose prose-sm max-w-none'>
                            <ReactMarkdown
                              remarkPlugins={[remarkGfm]}
                              components={{
                                p: ({ node, ...props }) => <p className='mb-2.5 text-sm text-gray-700 leading-relaxed' {...props} />,
                                h1: ({ node, ...props }) => <h1 className='text-base font-bold text-gray-900 mb-2 mt-0' {...props} />,
                                h2: ({ node, ...props }) => <h2 className='text-sm font-bold text-gray-900 mb-1.5 mt-3' {...props} />,
                                h3: ({ node, ...props }) => <h3 className='text-xs font-semibold text-gray-800 mb-1 mt-2' {...props} />,
                                ul: ({ node, ...props }) => <ul className='list-disc list-inside mb-2.5 text-sm text-gray-700 space-y-1' {...props} />,
                                ol: ({ node, ...props }) => <ol className='list-decimal list-inside mb-2.5 text-sm text-gray-700 space-y-1' {...props} />,
                                li: ({ node, ...props }) => <li className='text-sm text-gray-700' {...props} />,
                                code: ({ node, inline, ...props }) =>
                                  inline
                                    ? <code className='bg-white px-1.5 py-0.5 rounded text-xs text-gray-800 font-mono' {...props} />
                                    : <code className='block bg-white p-2.5 rounded text-xs text-gray-800 font-mono overflow-x-auto' {...props} />,
                                blockquote: ({ node, ...props }) => <blockquote className='border-l-2 border-gray-300 pl-3 italic text-sm text-gray-600 my-2.5' {...props} />
                              }}
                            >
                              {summary.content}
                            </ReactMarkdown>
                          </div>
                        </div>

                        {/* 底部信息条 - 扁平设计 */}
                        <div className='px-5 py-2.5 bg-white/50 flex items-center justify-between rounded-b-lg'>
                          <span className='text-xs text-gray-500 font-medium'>{summary.screenshotCount} {t('screenshots')}</span>
                          <span className='text-xs text-gray-400 tabular-nums'>{new Date(summary.createdAt).toLocaleTimeString(language === 'zh' ? 'zh-CN' : 'en-US', { hour: '2-digit', minute: '2-digit' })}</span>
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Footer: 统计信息 */}
      <footer className='px-8 py-3 bg-white border-t border-gray-100'>
        <div className='max-w-4xl mx-auto flex items-center justify-between'>
          <div className='flex items-center gap-8'>
            <div className='flex flex-col'>
              <span className='text-xs text-gray-400 font-medium uppercase tracking-wider mb-0.5'>{t('totalTime')}</span>
              <span className='text-sm font-bold text-gray-900 tabular-nums'>{totalHours}h</span>
            </div>
            <div className='w-px h-8 bg-gray-200' />
            <div className='flex flex-col'>
              <span className='text-xs text-gray-400 font-medium uppercase tracking-wider mb-0.5'>{t('activeHours')}</span>
              <span className='text-sm font-bold text-gray-900 tabular-nums'>{activeHours}</span>
            </div>
            <div className='w-px h-8 bg-gray-200' />
            <div className='flex flex-col'>
              <span className='text-xs text-gray-400 font-medium uppercase tracking-wider mb-0.5'>{t('focusScore')}</span>
              <span className='text-sm font-bold text-gray-900 tabular-nums'>{focusScore}%</span>
            </div>
          </div>
          <div className='flex flex-col items-end'>
            <span className='text-xs text-gray-400 font-medium uppercase tracking-wider mb-0.5'>{t('summaries')}</span>
            <span className='text-sm font-bold text-gray-900 tabular-nums'>{summaries.length}</span>
          </div>
        </div>
      </footer>
    </div>
  )
}
