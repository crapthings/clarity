import Sidebar from './Sidebar'
import TitleBar from './TitleBar'
import RecordButton from './RecordButton'
import Trace from '../pages/Trace'
import Summary from '../pages/Summary'
import Statistics from '../pages/Statistics'
import Settings from '../pages/Settings'
import { useAppStore } from '../store'

export default function MainLayout () {
  const currentPage = useAppStore((state) => state.currentPage)

  const renderPage = () => {
    switch (currentPage) {
      case 'trace':
        return <Trace />
      case 'summary':
        return <Summary />
      case 'statistics':
        return <Statistics />
      case 'settings':
        return <Settings />
      default:
        return <Trace />
    }
  }

  return (
    <div className='flex flex-col h-screen bg-gray-50 overflow-hidden'>
      <TitleBar />
      <div className='flex flex-1 overflow-hidden min-h-0'>
        <Sidebar />
        <main className='flex-1 overflow-y-auto bg-white relative min-w-0'>
          {renderPage()}
        </main>
      </div>
      {/* Footer Toolbar with Record Button */}
      <footer className='h-14 bg-white border-t border-gray-200 flex items-center justify-center px-4 shrink-0'>
        <RecordButton />
      </footer>
    </div>
  )
}
