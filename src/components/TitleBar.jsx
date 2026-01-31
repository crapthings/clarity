import { getCurrentWindow } from '@tauri-apps/api/window'

export default function TitleBar () {
  const appWindow = getCurrentWindow()

  const handleMinimize = () => {
    appWindow.minimize()
  }

  const handleMaximize = () => {
    appWindow.toggleMaximize()
  }

  const handleClose = () => {
    appWindow.close()
  }

  return (
    <div className='h-8 bg-gray-50 border-b border-gray-200 flex items-center justify-between px-4' data-tauri-drag-region>
      <div className='flex items-center gap-2' data-tauri-drag-region>
        <span className='text-xs font-medium text-gray-600'>Clarity</span>
      </div>
      <div className='flex items-center gap-1'>
        <button
          onClick={handleMinimize}
          className='w-6 h-6 flex items-center justify-center hover:bg-gray-200 rounded transition-colors'
          title='Minimize'
        >
          <svg
            xmlns='http://www.w3.org/2000/svg'
            width='12'
            height='12'
            viewBox='0 0 24 24'
            fill='none'
            stroke='currentColor'
            strokeWidth='2'
            strokeLinecap='round'
            strokeLinejoin='round'
            className='text-gray-600'
          >
            <path d='M5 12h14' />
          </svg>
        </button>
        <button
          onClick={handleMaximize}
          className='w-6 h-6 flex items-center justify-center hover:bg-gray-200 rounded transition-colors'
          title='Maximize'
        >
          <svg
            xmlns='http://www.w3.org/2000/svg'
            width='12'
            height='12'
            viewBox='0 0 24 24'
            fill='none'
            stroke='currentColor'
            strokeWidth='2'
            strokeLinecap='round'
            strokeLinejoin='round'
            className='text-gray-600'
          >
            <path d='M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3' />
          </svg>
        </button>
        <button
          onClick={handleClose}
          className='w-6 h-6 flex items-center justify-center hover:bg-red-500 hover:text-white rounded transition-colors'
          title='Close'
        >
          <svg
            xmlns='http://www.w3.org/2000/svg'
            width='12'
            height='12'
            viewBox='0 0 24 24'
            fill='none'
            stroke='currentColor'
            strokeWidth='2'
            strokeLinecap='round'
            strokeLinejoin='round'
            className='text-gray-600'
          >
            <path d='M18 6L6 18M6 6l12 12' />
          </svg>
        </button>
      </div>
    </div>
  )
}
