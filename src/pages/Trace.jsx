import { useState, useEffect, useMemo, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import Icon from '@mdi/react'
import { 
  mdiCheckCircle, 
  mdiAlert, 
  mdiPauseCircle, 
  mdiHelpCircle,
  mdiThumbUpOutline,
  mdiThumbDownOutline,
  mdiChevronDown,
  mdiChevronUp
} from '@mdi/js'
import { useTranslation } from '../i18n'
import { useAppStore } from '../store'

// 价值判断标签组件 - 使用 mdi 图标
function ValueLabel ({ value, language }) {
  const { t } = useTranslation()
  const labelMap = {
    high: { 
      text: t('highValue'), 
      icon: mdiCheckCircle,
      color: 'text-gray-600 bg-gray-50 border border-gray-200' 
    },
    low: { 
      text: t('lowEfficiency'), 
      icon: mdiAlert,
      color: 'text-gray-500 bg-gray-50 border border-gray-200' 
    },
    waiting: { 
      text: t('waiting'), 
      icon: mdiPauseCircle,
      color: 'text-gray-500 bg-gray-50 border border-gray-200' 
    },
    uncertain: { 
      text: t('uncertain'), 
      icon: mdiHelpCircle,
      color: 'text-gray-400 bg-gray-50 border border-gray-200' 
    }
  }

  const label = labelMap[value] || labelMap.uncertain

  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-normal ${label.color}`}>
      <Icon path={label.icon} size={0.75} />
      <span>{label.text}</span>
    </span>
  )
}

// 分析摘要内容，判断价值标签
function analyzeValueLabel (content) {
  const lowEfficiencyKeywords = ['distraction', 'inefficient', 'passive', 'browse', 'scroll', '分心', '低效', '被动', '浏览', '滚动']
  const waitingKeywords = ['waiting', 'idle', 'pause', '等待', '空闲', '暂停']
  const highValueKeywords = ['work', 'code', 'write', 'meeting', 'productive', 'focused', '工作', '代码', '写作', '会议', '高效', '专注']

  const lowerContent = content.toLowerCase()

  if (lowEfficiencyKeywords.some(kw => lowerContent.includes(kw))) {
    return 'low'
  }
  if (waitingKeywords.some(kw => lowerContent.includes(kw))) {
    return 'waiting'
  }
  if (highValueKeywords.some(kw => lowerContent.includes(kw))) {
    return 'high'
  }
  return 'uncertain'
}

// 提取摘要的简短描述（用于折叠状态）
function extractShortDescription (content) {
  // 尝试提取第一句话或前100个字符
  const firstSentence = content.split(/[。.\n]/)[0]
  if (firstSentence.length <= 100) {
    return firstSentence
  }
  return content.substring(0, 100) + '...'
}

export default function Trace () {
  const { t } = useTranslation()
  const language = useAppStore((state) => state.language || 'en')
  const [summaries, setSummaries] = useState([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)
  const [currentDate, setCurrentDate] = useState(new Date())
  const [expandedCards, setExpandedCards] = useState(new Set())
  const [screenshotPreviews, setScreenshotPreviews] = useState({})
  const [screenshotDataUrls, setScreenshotDataUrls] = useState({})
  const [heatmapHover, setHeatmapHover] = useState(null)
  const [selectedTag, setSelectedTag] = useState(null)
  const [viewMode, setViewMode] = useState('timeline') // timeline, tag, app, focus
  const [showSuggestions, setShowSuggestions] = useState(false)
  const mainRef = useRef(null)

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

  // 加载截图预览
  const loadScreenshotPreviews = async (summaryId, startTime, endTime) => {
    if (screenshotPreviews[summaryId]) {
      console.log('Screenshots already loaded for summary:', summaryId)
      return
    }

    try {
      console.log('Loading screenshot previews for summary:', summaryId, 'from', startTime, 'to', endTime)
      const traces = await invoke('get_traces', {
        startTime,
        endTime,
        limit: 5 // 只加载前5张作为预览
      })
      console.log('Loaded traces:', traces?.length || 0)
      if (traces && traces.length > 0) {
        console.log('First trace sample:', traces[0])
        console.log('File path:', traces[0].filePath || traces[0].file_path)
      }
      setScreenshotPreviews(prev => ({
        ...prev,
        [summaryId]: traces || []
      }))

      // 加载截图的 URLs（优先使用 convertFileSrc，失败则使用后端命令读取 base64）
      if (traces && traces.length > 0) {
        const dataUrls = {}
        for (const trace of traces.slice(0, 5)) {
          const filePath = trace.filePath || trace.file_path
          if (filePath) {
            try {
              // 优先使用 convertFileSrc（更快）
              const assetUrl = convertFileSrc(filePath)
              dataUrls[filePath] = assetUrl
              console.log('Converted file path to asset URL:', filePath, '->', assetUrl)
            } catch (err) {
              console.warn('convertFileSrc failed, trying backend command:', filePath, err)
              // 如果 convertFileSrc 失败，使用后端命令读取文件并转换为 base64
              try {
                const base64DataUrl = await invoke('read_screenshot_file', { filePath })
                dataUrls[filePath] = base64DataUrl
                console.log('Loaded file via backend command:', filePath)
              } catch (readErr) {
                console.error('Both convertFileSrc and backend command failed:', filePath, readErr)
                // 保留错误状态，让 UI 显示错误
              }
            }
          }
        }
        setScreenshotDataUrls(prev => ({
          ...prev,
          [summaryId]: dataUrls
        }))
      }
    } catch (err) {
      console.error('Failed to load screenshot previews:', err)
      // 即使失败也设置空数组，避免重复请求
      setScreenshotPreviews(prev => ({
        ...prev,
        [summaryId]: []
      }))
    }
  }

  useEffect(() => {
    loadSummaries(currentDate)
  }, [currentDate])

  const toggleCard = async (summaryId) => {
    const isCurrentlyExpanded = expandedCards.has(summaryId)
    
    if (isCurrentlyExpanded) {
      // 收起
      setExpandedCards(prev => {
        const newSet = new Set(prev)
        newSet.delete(summaryId)
        return newSet
      })
    } else {
      // 展开
      setExpandedCards(prev => {
        const newSet = new Set(prev)
        newSet.add(summaryId)
        return newSet
      })
      
      // 展开时加载截图预览
      const summary = summaries.find(s => s.id === summaryId)
      if (summary && !screenshotPreviews[summaryId]) {
        await loadScreenshotPreviews(summaryId, summary.startTime, summary.endTime)
      }
    }
  }

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

  // 跳转到指定时间
  const jumpToTime = (hour) => {
    const targetDate = new Date(currentDate)
    targetDate.setHours(hour, 0, 0, 0)
    // 找到对应时间的摘要并滚动到该位置
    const targetSummary = sortedSummaries.find(s => {
      const sTime = new Date(s.startTime)
      return sTime.getHours() === hour
    })
    if (targetSummary && mainRef.current) {
      const element = document.getElementById(`summary-${targetSummary.id}`)
      if (element) {
        element.scrollIntoView({ behavior: 'smooth', block: 'center' })
      }
    }
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
      return { entertainment: 0, productivity: 0, breakdown: {} }
    }

    const entertainmentKeywords = ['video', 'game', 'entertainment', 'watch', 'play', 'movie', 'music', 'social', 'browse', 'fun', 'relax', '视频', '游戏', '娱乐', '观看', '播放', '电影', '音乐', '社交', '浏览', '放松']
    const productivityKeywords = ['work', 'code', 'write', 'read', 'study', 'learn', 'meeting', 'email', 'document', 'project', 'task', '工作', '代码', '写作', '阅读', '学习', '会议', '邮件', '文档', '项目', '任务']

    let entertainmentScore = 0
    let productivityScore = 0
    const breakdown = {
      video: 0,
      browse: 0,
      work: 0,
      code: 0,
      other: 0
    }

    summaries.forEach(summary => {
      const content = summary.content.toLowerCase()
      const entertainmentMatches = entertainmentKeywords.filter(keyword => content.includes(keyword.toLowerCase())).length
      const productivityMatches = productivityKeywords.filter(keyword => content.includes(keyword.toLowerCase())).length

      entertainmentScore += entertainmentMatches
      productivityScore += productivityMatches

      // 详细分类
      if (content.includes('video') || content.includes('视频')) breakdown.video++
      else if (content.includes('browse') || content.includes('浏览')) breakdown.browse++
      else if (content.includes('work') || content.includes('工作')) breakdown.work++
      else if (content.includes('code') || content.includes('代码')) breakdown.code++
      else breakdown.other++
    })

    const total = entertainmentScore + productivityScore
    if (total === 0) {
      return { entertainment: 0, productivity: 0, breakdown: {} }
    }

    return {
      entertainment: Math.round((entertainmentScore / total) * 100),
      productivity: Math.round((productivityScore / total) * 100),
      breakdown
    }
  }, [summaries])

  // 标签颜色映射
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

  // 生成一句话总结
  const oneLinerSummary = useMemo(() => {
    if (summaries.length === 0) return null

    const productivityPercent = calculateHeatmap.productivity
    const entertainmentPercent = calculateHeatmap.entertainment

    if (language === 'zh') {
      if (productivityPercent > 70) {
        return '今天有效工作时间集中在上午，整体节奏良好。'
      } else if (entertainmentPercent > 70) {
        return '今天娱乐时间较多，主要集中在下午时段。'
      } else {
        return '今天工作与娱乐时间分布较为均衡。'
      }
    } else {
      if (productivityPercent > 70) {
        return 'Effective work concentrated in the morning, overall rhythm is good.'
      } else if (entertainmentPercent > 70) {
        return 'More entertainment time today, mainly in the afternoon.'
      } else {
        return 'Work and entertainment time are relatively balanced today.'
      }
    }
  }, [summaries, calculateHeatmap, language])

  // AI建议
  const aiSuggestions = useMemo(() => {
    if (summaries.length === 0) return []

    const suggestions = []
    const productivityPercent = calculateHeatmap.productivity

    if (language === 'zh') {
      if (productivityPercent < 50) {
        suggestions.push('建议明天将重要任务安排在专注度较高的时段（通常是上午）。')
        suggestions.push('下午可以安排低负载任务或明确休息时间。')
      } else {
        suggestions.push('保持当前的工作节奏，可以尝试将分析类任务放在效率较高的时段。')
      }
    } else {
      if (productivityPercent < 50) {
        suggestions.push('Consider scheduling important tasks during your most focused hours (usually morning).')
        suggestions.push('Afternoon can be reserved for low-load tasks or clear rest time.')
      } else {
        suggestions.push('Maintain current work rhythm, try scheduling analytical tasks during peak efficiency hours.')
      }
    }

    return suggestions
  }, [summaries, calculateHeatmap, language])

  const sortedSummaries = useMemo(() => {
    let sorted = [...summaries].sort((a, b) => {
      return new Date(b.startTime) - new Date(a.startTime)
    })

    // 根据视图模式过滤/排序
    if (selectedTag) {
      sorted = sorted.filter(s => {
        const content = s.content.toLowerCase()
        return content.includes(selectedTag.toLowerCase())
      })
    }

    if (viewMode === 'focus') {
      sorted.sort((a, b) => {
        const aValue = analyzeValueLabel(a.content)
        const bValue = analyzeValueLabel(b.content)
        const order = { high: 0, uncertain: 1, waiting: 2, low: 3 }
        return order[aValue] - order[bValue]
      })
    }

    return sorted
  }, [summaries, selectedTag, viewMode])

  // 获取时间轴上的小时标记
  const timelineHours = useMemo(() => {
    const hours = new Set()
    summaries.forEach(summary => {
      const start = new Date(summary.startTime)
      hours.add(start.getHours())
    })
    return Array.from(hours).sort((a, b) => a - b)
  }, [summaries])

  return (
    <div className='flex flex-col h-full bg-white'>
      {/* Header */}
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

      {/* Sub Header: 热度表 + 行为标签 + 视图切换 */}
      <div className='px-8 py-4 bg-white border-b border-gray-100'>
        {summaries.length === 0 && !loading
          ? (
              <div className='text-center py-2'>
                <span className='text-sm font-medium text-gray-600'>{t('whatAWonderfulDay')}</span>
              </div>
            )
          : (
              <>
                {/* 热度表 - 可hover显示详细构成 */}
                <div className='mb-3'>
                  <div className='flex items-center justify-between mb-2'>
                    <div
                      className='flex items-center gap-1.5 cursor-help relative'
                      onMouseEnter={() => setHeatmapHover('entertainment')}
                      onMouseLeave={() => setHeatmapHover(null)}
                    >
                      <div className='w-1.5 h-1.5 rounded-full bg-gray-300' />
                      <span className='text-xs font-medium text-gray-500 uppercase tracking-wider'>{t('entertainment')}</span>
                      <span className='text-xs text-gray-400 font-semibold tabular-nums'>{calculateHeatmap.entertainment}%</span>
                      {heatmapHover === 'entertainment' && (
                        <div className='absolute top-full left-0 mt-1 px-2 py-1 bg-gray-900 text-white text-xs rounded shadow-lg z-10 whitespace-nowrap'>
                          {t('breakdown')}: {calculateHeatmap.breakdown.video}% {language === 'zh' ? '视频' : 'Video'}, {calculateHeatmap.breakdown.browse}% {language === 'zh' ? '浏览' : 'Browse'}
                        </div>
                      )}
                    </div>
                    <div
                      className='flex items-center gap-1.5 cursor-help relative'
                      onMouseEnter={() => setHeatmapHover('productivity')}
                      onMouseLeave={() => setHeatmapHover(null)}
                    >
                      <span className='text-xs text-gray-400 font-semibold tabular-nums'>{calculateHeatmap.productivity}%</span>
                      <span className='text-xs font-medium text-gray-500 uppercase tracking-wider'>{t('productivity')}</span>
                      <div className='w-1.5 h-1.5 rounded-full bg-gray-900' />
                      {heatmapHover === 'productivity' && (
                        <div className='absolute top-full right-0 mt-1 px-2 py-1 bg-gray-900 text-white text-xs rounded shadow-lg z-10 whitespace-nowrap'>
                          {t('breakdown')}: {calculateHeatmap.breakdown.work}% {language === 'zh' ? '工作' : 'Work'}, {calculateHeatmap.breakdown.code}% {language === 'zh' ? '代码' : 'Code'}
                        </div>
                      )}
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

                {/* 行为标签 + 视图切换 */}
                <div className='flex items-center justify-between gap-4'>
                  {activityTags.length > 0 && (
                      <div className='flex items-center gap-2.5 flex-1'>
                        <span className='text-xs font-semibold text-gray-400 uppercase tracking-wider whitespace-nowrap flex-shrink-0'>{t('activityTags')}</span>
                        <div className='flex gap-1.5 overflow-x-auto scrollbar-hide flex-1'>
                          {activityTags.map((tag, index) => {
                            const bgColor = tagColorMap[tag] || 'bg-gray-300'
                            const isSelected = selectedTag === tag
                            return (
                              <button
                                key={index}
                                onClick={() => setSelectedTag(isSelected ? null : tag)}
                                className={`px-2.5 py-1 ${bgColor} rounded-md text-xs font-medium text-gray-800 whitespace-nowrap flex-shrink-0 transition-all ${
                                  isSelected ? 'ring-2 ring-gray-900' : ''
                                }`}
                              >
                                {tag}
                              </button>
                            )
                          })}
                        </div>
                      </div>
                    )}
                  {/* 视图切换按钮 */}
                  <div className='flex items-center gap-1.5 flex-shrink-0'>
                    <button
                      onClick={() => {
                        setViewMode('timeline')
                        setSelectedTag(null)
                      }}
                      className={`px-2 py-1 text-xs font-medium rounded transition-all ${
                        viewMode === 'timeline' ? 'bg-gray-900 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                      }`}
                    >
                      {t('backToTimeline')}
                    </button>
                    <button
                      onClick={() => setViewMode('focus')}
                      className={`px-2 py-1 text-xs font-medium rounded transition-all ${
                        viewMode === 'focus' ? 'bg-gray-900 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                      }`}
                    >
                      {t('sortByFocus')}
                    </button>
                  </div>
                </div>

                {/* 时间跳转导航 */}
                {timelineHours.length > 0 && (
                    <div className='mt-3 flex items-center gap-2'>
                      <span className='text-xs text-gray-400'>{t('timeNavigation')}:</span>
                      <div className='flex gap-1'>
                        {timelineHours.map(hour => (
                          <button
                            key={hour}
                            onClick={() => jumpToTime(hour)}
                            className='px-2 py-0.5 text-xs font-medium text-gray-600 hover:bg-gray-100 rounded transition-all'
                            title={t('clickToJump')}
                          >
                            {hour}:00
                          </button>
                        ))}
                      </div>
                    </div>
                  )}
              </>
            )}
      </div>

      {/* Main Content: 时间轴列表 */}
      <main ref={mainRef} className='flex-1 overflow-y-auto px-8 py-6'>
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
                  const isExpanded = expandedCards.has(summary.id)
                  const valueLabel = analyzeValueLabel(summary.content)
                  const shortDesc = extractShortDescription(summary.content)
                  const previews = screenshotPreviews[summary.id] || []

                  return (
                    <div key={summary.id} id={`summary-${summary.id}`} className='relative pl-8 pb-6'>
                      {/* 时间标记 */}
                      <div className='absolute left-0 top-2 -translate-x-1/2'>
                        <div className='relative flex items-center'>
                          <div className='relative'>
                            <div className='w-2.5 h-2.5 rounded-full bg-gray-900 ring-2 ring-white shadow-sm z-10' />
                            {isFirst && (
                              <div className='absolute top-0 left-0 w-2.5 h-2.5 rounded-full bg-gray-900 animate-ping opacity-75' />
                            )}
                          </div>
                          <div className='absolute right-full mr-3 flex items-center justify-center'>
                            <div className='text-xs font-semibold text-gray-900 tabular-nums whitespace-nowrap'>
                              {startTime}
                            </div>
                          </div>
                        </div>
                      </div>

                      {/* 内容卡片 */}
                      <div className='bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors'>
                        <div className='p-5'>
                          {/* 卡片头部：价值标签 + 展开按钮 */}
                          <div className='flex items-start justify-between gap-3 mb-3'>
                            <div className='flex-1'>
                              {!isExpanded && (
                                <div className='text-sm text-gray-700 leading-relaxed'>{shortDesc}</div>
                              )}
                              {isExpanded && (
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
                              )}
                            </div>
                            <div className='flex items-start gap-2 flex-shrink-0'>
                              <ValueLabel value={valueLabel} language={language} />
                              <button
                                onClick={() => toggleCard(summary.id)}
                                className='flex items-center gap-1 px-2 py-1 text-xs font-medium text-gray-600 hover:bg-gray-200 rounded transition-all'
                              >
                                {isExpanded ? (
                                  <>
                                    <Icon path={mdiChevronUp} size={0.75} />
                                    <span>{t('collapse')}</span>
                                  </>
                                ) : (
                                  <>
                                    <Icon path={mdiChevronDown} size={0.75} />
                                    <span>{t('expand')}</span>
                                  </>
                                )}
                              </button>
                            </div>
                          </div>

                          {/* 展开状态：截图预览 */}
                          {isExpanded && (
                              <div className='mt-4 pt-4 border-t border-gray-200'>
                                {previews.length > 0 ? (
                                  <>
                                    <div className='text-xs font-medium text-gray-500 mb-2'>{t('viewScreenshots')} ({previews.length})</div>
                                    <div className='grid grid-cols-5 gap-2'>
                                      {previews.slice(0, 5).map((trace, idx) => {
                                        // 使用 convertFileSrc 转换文件路径（注意字段名是 filePath，因为 serde 会转换为 camelCase）
                                        const filePath = trace.filePath || trace.file_path
                                        if (!filePath) {
                                          console.warn('Trace missing filePath:', trace)
                                          return (
                                            <div key={idx} className='aspect-video bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400'>
                                              No path
                                            </div>
                                          )
                                        }
                                        
                                        // 获取已加载的 data URL
                                        const dataUrls = screenshotDataUrls[summary.id] || {}
                                        let imageSrc = dataUrls[filePath]
                                        
                                        // 如果还没有加载，显示加载状态（实际加载在 loadScreenshotPreviews 中完成）
                                        if (!imageSrc) {
                                          return (
                                            <div key={idx} className='aspect-video bg-gray-200 rounded flex items-center justify-center text-xs text-gray-400'>
                                              {language === 'zh' ? '加载中...' : 'Loading...'}
                                            </div>
                                          )
                                        }
                                        
                                        return (
                                          <div key={idx} className='aspect-video bg-gray-200 rounded overflow-hidden cursor-pointer hover:opacity-80 transition-opacity relative'>
                                            <img
                                              src={imageSrc}
                                              alt={`Screenshot ${idx + 1}`}
                                              className='w-full h-full object-cover'
                                              onError={(e) => {
                                                console.error('Failed to load screenshot image:', {
                                                  originalPath: filePath,
                                                  imageSrc: imageSrc,
                                                  error: e
                                                })
                                                // 显示错误信息
                                                const parent = e.target.parentElement
                                                if (parent) {
                                                  parent.innerHTML = `<div class="flex flex-col items-center justify-center h-full text-xs text-gray-400 p-2">
                                                    <div>Failed</div>
                                                    <div class="text-[10px] mt-1 break-all opacity-50">${filePath.substring(Math.max(0, filePath.length - 30))}</div>
                                                  </div>`
                                                }
                                              }}
                                              onLoad={() => {
                                                console.log('Successfully loaded screenshot:', filePath)
                                              }}
                                            />
                                          </div>
                                        )
                                      })}
                                    </div>
                                  </>
                                ) : (
                                  <div className='text-xs text-gray-400 italic'>{language === 'zh' ? '加载截图中...' : 'Loading screenshots...'}</div>
                                )}
                              </div>
                            )}

                          {/* AI 推测标注 */}
                          <div className='mt-3 pt-3 border-t border-gray-200'>
                            <div className='text-xs text-gray-400 italic'>
                              {t('aiInference')}: {t('aiInferenceNote')}
                            </div>
                          </div>

                          {/* 用户反馈按钮 - 使用 mdi 图标 */}
                          <div className='mt-3 flex items-center gap-1.5'>
                            <button 
                              className='p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded transition-all'
                              title={language === 'zh' ? '正确' : 'Correct'}
                            >
                              <Icon path={mdiThumbUpOutline} size={1} />
                            </button>
                            <button 
                              className='p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded transition-all'
                              title={language === 'zh' ? '不正确' : 'Incorrect'}
                            >
                              <Icon path={mdiThumbDownOutline} size={1} />
                            </button>
                            <button 
                              className='px-2 py-1 text-xs font-normal text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded transition-all'
                            >
                              {t('correctLabel')}
                            </button>
                          </div>
                        </div>

                        {/* 底部信息条 */}
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

      {/* Footer: 统计信息 + 一句话总结 + AI建议 */}
      <footer className='px-8 py-3 bg-white border-t border-gray-100'>
        <div className='max-w-4xl mx-auto space-y-3'>
          {/* 统计信息 */}
          <div className='flex items-center justify-between'>
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

          {/* 一句话总结 */}
          {oneLinerSummary && (
              <div className='pt-3 border-t border-gray-200'>
                <div className='text-xs font-medium text-gray-400 uppercase tracking-wider mb-1'>{t('oneLinerSummary')}</div>
                <div className='text-sm text-gray-900 font-medium'>{oneLinerSummary}</div>
              </div>
            )}

          {/* AI建议（可折叠） */}
          {aiSuggestions.length > 0 && (
              <div className='pt-3 border-t border-gray-200'>
                <button
                  onClick={() => setShowSuggestions(!showSuggestions)}
                  className='flex items-center justify-between w-full text-left'
                >
                  <div className='text-xs font-medium text-gray-400 uppercase tracking-wider'>{t('aiSuggestions')}</div>
                  <svg
                    className={`w-4 h-4 text-gray-400 transition-transform ${showSuggestions ? 'rotate-180' : ''}`}
                    fill='none'
                    stroke='currentColor'
                    viewBox='0 0 24 24'
                  >
                    <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={2} d='M19 9l-7 7-7-7' />
                  </svg>
                </button>
                {showSuggestions && (
                    <div className='mt-2 space-y-1'>
                      {aiSuggestions.map((suggestion, idx) => (
                        <div key={idx} className='text-sm text-gray-700'>• {suggestion}</div>
                      ))}
                    </div>
                  )}
              </div>
            )}
        </div>
      </footer>
    </div>
  )
}
