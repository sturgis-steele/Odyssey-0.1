import { useCallback, useEffect, useRef, useState } from 'react'
import type { Status } from '../types'
import { api } from '../services/api'

export function useSystem() {
  const [status, setStatus] = useState<Status | null>(null)
  const [checked, setChecked] = useState(false)
  const [checking, setChecking] = useState(false)
  const inFlight = useRef(false)
  const mounted = useRef(true)
  const refresh = useCallback(async () => {
    if (inFlight.current) return
    inFlight.current = true
    setChecking(true)
    try {
      const next = await api.status()
      if (mounted.current) setStatus(next)
    } catch {
      if (mounted.current) setStatus(null)
    } finally {
      inFlight.current = false
      if (mounted.current) {
        setChecked(true)
        setChecking(false)
      }
    }
  }, [])
  useEffect(() => {
    mounted.current = true
    void refresh()
    const visible = () => {
      if (!document.hidden) void refresh()
    }
    const timer = window.setInterval(visible, 30000)
    document.addEventListener('visibilitychange', visible)
    window.addEventListener('focus', visible)
    return () => {
      mounted.current = false
      window.clearInterval(timer)
      document.removeEventListener('visibilitychange', visible)
      window.removeEventListener('focus', visible)
    }
  }, [refresh])
  return { status, checked, checking, refresh }
}
export type System = ReturnType<typeof useSystem>
