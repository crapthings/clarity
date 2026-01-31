import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

function ScreenshotController () {
  const [isRecording, setIsRecording] = useState(false)
  const [screenshotsCount, setScreenshotsCount] = useState(0)
  const [storagePath, setStoragePath] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)
  const [testResult, setTestResult] = useState(null)
  const [testing, setTesting] = useState(false)

  useEffect(() => {
    // 初始化时获取状态和存储路径
    loadStatus()
    loadStoragePath()
    
    // 每秒更新一次状态
    const interval = setInterval(() => {
      if (isRecording) {
        loadStatus()
      }
    }, 1000)

    return () => clearInterval(interval)
  }, [isRecording])

  const loadStatus = async () => {
    try {
      const status = await invoke('get_status')
      setIsRecording(status.is_recording)
      setScreenshotsCount(status.screenshots_count)
      setStoragePath(status.storage_path)
    } catch (err) {
      console.error('Failed to load status:', err)
      setError(err.toString())
    }
  }

  const loadStoragePath = async () => {
    try {
      const path = await invoke('get_storage_path')
      setStoragePath(path)
    } catch (err) {
      console.error('Failed to load storage path:', err)
    }
  }

  const handleStartRecording = async () => {
    setLoading(true)
    setError(null)
    try {
      const status = await invoke('start_recording')
      setIsRecording(status.is_recording)
      setScreenshotsCount(status.screenshots_count)
      setStoragePath(status.storage_path)
    } catch (err) {
      console.error('Failed to start recording:', err)
      setError(err.toString())
    } finally {
      setLoading(false)
    }
  }

  const handleStopRecording = async () => {
    setLoading(true)
    setError(null)
    try {
      const status = await invoke('stop_recording')
      setIsRecording(status.is_recording)
      setScreenshotsCount(status.screenshots_count)
    } catch (err) {
      console.error('Failed to stop recording:', err)
      setError(err.toString())
    } finally {
      setLoading(false)
    }
  }

  const handleTestScreenshot = async () => {
    setTesting(true)
    setError(null)
    setTestResult(null)
    try {
      const result = await invoke('test_screenshot')
      setTestResult(result)
    } catch (err) {
      console.error('Failed to test screenshot:', err)
      setError(err.toString())
      setTestResult(null)
    } finally {
      setTesting(false)
    }
  }

  return (
    <div className='screenshot-controller mb-6'>
      <h2 className='text-xl font-semibold text-gray-900 mb-4'>Screenshot Recording</h2>
      
      {error && (
        <div className='mb-4 p-4 bg-gray-50 border border-gray-200 text-gray-700 rounded-lg'>
          {error}
        </div>
      )}

      {/* macOS 权限提示 */}
      <div className='mb-6 p-4 bg-gray-50 border border-gray-200 rounded-lg'>
        <h3 className='font-semibold text-gray-900 mb-2'>macOS 屏幕录制权限</h3>
        <p className='text-sm text-gray-700 mb-2'>
          如果截图只显示桌面或应用窗口，请确保已授予屏幕录制权限：
        </p>
        <ol className='text-sm text-gray-700 list-decimal list-inside space-y-1 mb-3'>
          <li>打开 <strong>系统设置</strong> → <strong>隐私与安全性</strong> → <strong>屏幕录制</strong></li>
          <li>找到 <strong>clarity</strong> 并启用</li>
          <li>重启应用以使权限生效</li>
        </ol>
        <button
          onClick={handleTestScreenshot}
          disabled={testing}
          className='px-4 py-2 bg-gray-900 hover:bg-gray-800 text-white text-sm rounded-lg disabled:opacity-50 transition-colors font-medium'
        >
          {testing ? '测试中...' : '测试截图功能'}
        </button>
        {testResult && (
          <div className='mt-3 p-3 bg-white border border-gray-200 rounded text-sm text-gray-800'>
            {testResult}
          </div>
        )}
      </div>

      <div className='mb-6'>
        <div className='flex items-center gap-4 mb-4'>
          <button
            onClick={isRecording ? handleStopRecording : handleStartRecording}
            disabled={loading}
            className={`px-6 py-2.5 rounded-lg font-medium transition-colors ${
              isRecording
                ? 'bg-gray-900 hover:bg-gray-800 text-white'
                : 'bg-gray-900 hover:bg-gray-800 text-white'
            } disabled:opacity-50 disabled:cursor-not-allowed`}
          >
            {loading
              ? 'Processing...'
              : isRecording
                ? 'Stop Recording'
                : 'Start Recording'}
          </button>

          <div className='flex items-center gap-2'>
            <div
              className={`w-2 h-2 rounded-full ${
                isRecording ? 'bg-gray-900 animate-pulse' : 'bg-gray-400'
              }`}
            />
            <span className='text-sm text-gray-600 font-medium'>
              {isRecording ? 'Recording' : 'Stopped'}
            </span>
          </div>
        </div>

        <div className='space-y-2 text-sm text-gray-600'>
          <div>
            <span className='font-medium text-gray-700'>Screenshots captured:</span>{' '}
            <span className='text-gray-900'>{screenshotsCount.toLocaleString()}</span>
          </div>
          <div>
            <span className='font-medium text-gray-700'>Storage path:</span>{' '}
            <code className='bg-gray-100 px-2 py-1 rounded text-xs text-gray-800'>
              {storagePath || 'Loading...'}
            </code>
          </div>
          <div className='text-xs text-gray-500 mt-2'>
            Recording at 1 FPS (1 screenshot per second)
          </div>
        </div>
      </div>
    </div>
  )
}

export default ScreenshotController