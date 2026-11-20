import { useCallback, useEffect, useRef, useState } from 'react'
import { api, readable } from '../services/api'
import type { ChatEvent, Summary, Turn } from '../types'

function updateTurn(turn: Turn, event: ChatEvent): Turn {
  switch (event.type) {
    case 'message_start':
      return { ...turn, id: event.turn_id }
    case 'text_delta':
      return { ...turn, assistant: turn.assistant + event.text }
    case 'tool_start':
      return {
        ...turn,
        tools: [...turn.tools, { call_id: event.call_id, name: event.name, ok: null, result: {} }],
      }
    case 'tool_result':
      return { ...turn, tools: turn.tools.map((t) => (t.call_id === event.call_id ? event : t)) }
    case 'message_complete':
      return { ...turn, status: 'complete' }
    case 'error':
      return { ...turn, status: 'error', error: event.message }
  }
}

export function useChat(onFailure: () => void) {
  const [conversation, setConversation] = useState<Summary | null>(null)
  const [turns, setTurns] = useState<Turn[]>([])
  const [before, setBefore] = useState<number | null>(null)
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const controller = useRef<AbortController | null>(null)
  const operation = useRef(0)

  useEffect(
    () => () => {
      operation.current++
      controller.current?.abort()
    },
    [],
  )

  const open = useCallback(async (id: string) => {
    if (controller.current) return false
    const mark = ++operation.current
    setLoading(true)
    setError('')
    try {
      const detail = await api.detail(id)
      if (mark !== operation.current) return false
      setConversation(detail)
      setTurns(detail.turns)
      setBefore(detail.next_before)
      setDraft('')
      return true
    } catch (err) {
      if (mark === operation.current) setError(readable(err))
      return false
    } finally {
      if (mark === operation.current) setLoading(false)
    }
  }, [])

  function fresh() {
    if (controller.current) return
    operation.current++
    setLoading(false)
    setConversation(null)
    setTurns([])
    setBefore(null)
    setDraft('')
    setError('')
  }

  async function older() {
    if (!conversation || !before || loading || busy) return
    const mark = operation.current
    setLoading(true)
    try {
      const detail = await api.detail(conversation.id, before)
      if (mark === operation.current) {
        setTurns((items) => [...detail.turns, ...items])
        setBefore(detail.next_before)
      }
    } catch (err) {
      if (mark === operation.current) setError(readable(err))
    } finally {
      if (mark === operation.current) setLoading(false)
    }
  }

  async function send() {
    const message = draft.trim()
    if (!message || controller.current || loading) return
    const stream = new AbortController()
    controller.current = stream
    const mark = ++operation.current
    setBusy(true)
    setError('')
    setDraft('')
    let selected = conversation
    let started = false
    let optimistic = false
    try {
      if (!selected) selected = await api.create()
      if (mark !== operation.current || stream.signal.aborted) return
      setConversation(selected)
      const pending: Turn = {
        seq: 0,
        id: 'pending',
        user: message,
        assistant: '',
        created_at: new Date().toISOString(),
        tools: [],
        status: 'generating',
        error: null,
      }
      setTurns((items) => [...items, pending])
      optimistic = true
      await api.chat(selected.id, message, stream.signal, (event) => {
        if (mark !== operation.current) return
        if (event.type === 'message_start') started = true
        if (event.type === 'error') onFailure()
        setTurns((items) => [...items.slice(0, -1), updateTurn(items[items.length - 1], event)])
      })
    } catch (err) {
      if (mark === operation.current) {
        const detail = stream.signal.aborted ? 'Generation stopped.' : readable(err)
        if (!started) {
          setDraft(message)
          setError(detail)
          if (optimistic) setTurns((items) => items.slice(0, -1))
        } else {
          setTurns((items) => [
            ...items.slice(0, -1),
            {
              ...items[items.length - 1],
              status: stream.signal.aborted ? 'interrupted' : 'error',
              error: detail,
            },
          ])
        }
        if (!stream.signal.aborted) onFailure()
      }
    } finally {
      if (mark === operation.current) {
        if (selected) {
          try {
            const detail = await api.detail(selected.id)
            if (mark === operation.current) {
              setConversation(detail)
              if (!detail.turns.some((t) => t.status === 'generating')) {
                setTurns(detail.turns)
                setBefore(detail.next_before)
              }
            }
          } catch {
            /* Keep the last visible response while the backend is unavailable. */
          }
        }
        if (!started && stream.signal.aborted) setDraft(message)
        controller.current = null
        setBusy(false)
      }
    }
  }

  // A conversation can be reopened while another tab is still generating.
  const watching = !busy && Boolean(conversation) && turns.some((turn) => turn.status === 'generating')
  useEffect(() => {
    if (!watching || !conversation) return
    const mark = operation.current
    const id = conversation.id
    let active = true
    let timer = window.setTimeout(poll, 1200)
    async function poll() {
      if (!active || mark !== operation.current) return
      if (document.hidden) {
        timer = window.setTimeout(poll, 4000)
        return
      }
      try {
        const detail = await api.detail(id)
        if (!active || mark !== operation.current) return
        setTurns(detail.turns)
        setBefore(detail.next_before)
        setError('')
        if (detail.turns.some((turn) => turn.status === 'generating')) {
          timer = window.setTimeout(poll, 1200)
        }
      } catch (err) {
        if (!active || mark !== operation.current) return
        setError(readable(err))
        timer = window.setTimeout(poll, 4000)
      }
    }
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [watching, conversation])

  return {
    conversation,
    turns,
    before,
    draft,
    setDraft,
    busy,
    loading,
    error,
    open,
    older,
    fresh,
    send,
    stop: () => controller.current?.abort(),
    watching,
  }
}
export type Chat = ReturnType<typeof useChat>
