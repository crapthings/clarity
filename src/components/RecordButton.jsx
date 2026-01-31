import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useTranslation } from '../i18n'

export default function RecordButton () {
  const { t } = useTranslation()
  const [isRecording, setIsRecording] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)

  const loadStatus = async () => {
    try {
      const status = await invoke('get_status')
      setIsRecording(status.is_recording)
      setError(null)
    } catch (err) {
      console.error('Failed to load status:', err)
      setError(err?.toString() || String(err))
    }
  }

  useEffect(() => {
    loadStatus()
    const interval = setInterval(loadStatus, 1000)
    return () => clearInterval(interval)
  }, [])

  const handleToggle = async () => {
    if (loading) return

    setLoading(true)
    setError(null)
    try {
      if (isRecording) {
        console.log('Stopping recording...')
        const result = await invoke('stop_recording')
        console.log('Stop recording result:', result)
      } else {
        console.log('Starting recording...')
        const result = await invoke('start_recording')
        console.log('Start recording result:', result)
      }
      // 等待一下再刷新状态，确保后端处理完成
      setTimeout(async () => {
        await loadStatus()
      }, 500)
    } catch (err) {
      console.error('Failed to toggle recording:', err)
      setError(err?.toString() || String(err))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className='flex items-center justify-center gap-2'>
      {error && (
        <div className='text-xs text-red-600 bg-red-50 px-2 py-0.5 rounded border border-red-200'>
          {error}
        </div>
      )}
      <button
        onClick={handleToggle}
        disabled={loading}
        className={`
          w-10 h-10 rounded-full
          flex items-center justify-center
          transition-all duration-200
          shadow-sm hover:shadow-md active:scale-95
          disabled:opacity-50 disabled:cursor-not-allowed
          ${
            isRecording
              ? 'bg-red-500 hover:bg-red-600 active:bg-red-700'
              : 'bg-gray-900 hover:bg-gray-800 active:bg-gray-700'
          }
        `}
        title={
          isRecording
            ? t('stopRecording')
            : t('startRecording')
        }
      >
        {loading
          ? (
            <div className='w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full animate-spin' />
          )
          : isRecording
            ? (
              <div className='w-3 h-3 bg-white rounded-sm' />
            )
            : (
              <div className='w-0 h-0 border-l-8 border-l-white border-t-[6px] border-t-transparent border-b-[6px] border-b-transparent ml-0.5' />
            )}
      </button>
      <div className='flex items-center gap-1.5'>
        <div
          className={`w-1.5 h-1.5 rounded-full ${
            isRecording ? 'bg-red-500 animate-pulse' : 'bg-gray-400'
          }`}
        />
        <span className='text-xs font-medium text-gray-600'>
          {loading
            ? t('processing')
            : isRecording
              ? t('recording')
              : t('stopped')}
        </span>
      </div>
    </div>
  )
}
