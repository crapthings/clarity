import { useMemo } from 'react'
import Icon from '@mdi/react'
import { mdiTimeline, mdiFileDocumentOutline, mdiChartBox, mdiCog, mdiEyeOutline } from '@mdi/js'
import { useAppStore } from '../store'
import { useTranslation } from '../i18n'

export default function Sidebar () {
  const { t, language } = useTranslation()
  const currentPage = useAppStore((state) => state.currentPage)
  const setCurrentPage = useAppStore((state) => state.setCurrentPage)

  // 使用 useMemo 确保语言变化时重新计算
  const navItems = useMemo(() => [
    { id: 'trace', label: t('navTrace'), icon: mdiTimeline },
    { id: 'summary', label: t('navSummary'), icon: mdiFileDocumentOutline },
    { id: 'statistics', label: t('navStatistics'), icon: mdiChartBox },
    { id: 'settings', label: t('navSettings'), icon: mdiCog }
  ], [t, language])

  return (
    <aside className='w-56 bg-gray-50 border-r border-gray-200 h-full flex flex-col'>
      <div className='p-4 border-b border-gray-200 flex items-center gap-2'>
        <div className='w-8 h-8 rounded-lg bg-gray-900 flex items-center justify-center'>
          <Icon path={mdiEyeOutline} size={1.2} className='text-white' />
        </div>
        <div className='flex flex-col'>
          <h1 className='text-sm font-semibold text-gray-900 leading-tight'>Clarity</h1>
          <span className='text-xs text-gray-500 leading-tight'>{t('appSubtitle')}</span>
        </div>
      </div>

      <nav className='flex-1 p-3'>
        <ul className='space-y-0.5'>
          {navItems.map((item) => (
            <li key={item.id}>
              <button
                onClick={() => setCurrentPage(item.id)}
                className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md transition-all ${
                  currentPage === item.id
                    ? 'bg-gray-900 text-white shadow-sm'
                    : 'text-gray-700 hover:bg-gray-100 hover:text-gray-900'
                }`}
              >
                <Icon path={item.icon} size={1.1} className={currentPage === item.id ? 'text-white' : 'text-gray-600'} />
                <span className='font-medium text-xs'>{item.label}</span>
              </button>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  )
}
