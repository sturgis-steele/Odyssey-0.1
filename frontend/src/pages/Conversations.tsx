import { useCallback, useEffect, useState } from 'react'
import { ArrowRight, History, MessageSquare, RotateCw, Trash2 } from 'lucide-react'
import type { Summary } from '../types'
import { api, readable } from '../services/api'

export function Conversations({
  open,
  removed,
  busy,
  online,
}: {
  open: (id: string) => Promise<boolean>
  removed: (id: string) => void
  busy: boolean
  online: boolean
}) {
  const [items, setItems] = useState<Summary[]>([])
  const [more, setMore] = useState(false)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [confirm, setConfirm] = useState('')
  const load = useCallback(async (offset = 0) => {
    setLoading(true)
    setError('')
    try {
      const list = await api.list(offset)
      setItems((prev) =>
        offset ? [...prev, ...list.filter((v) => !prev.some((p) => p.id === v.id))] : list,
      )
      setMore(list.length === 30)
    } catch (err) {
      setError(readable(err))
    } finally {
      setLoading(false)
    }
  }, [])
  useEffect(() => {
    void load()
  }, [load, online])

  async function remove(id: string) {
    setLoading(true)
    try {
      await api.remove(id)
      removed(id)
      setConfirm('')
      await load()
    } catch (err) {
      setError(readable(err))
      setLoading(false)
    }
  }
  return (
    <section className="page secondary-page">
      <header className="page-heading">
        <h1>Conversations</h1>
        <button className="secondary" disabled={loading} onClick={() => load()}>
          <RotateCw size={16} />
          Refresh
        </button>
      </header>
      <div className="page-content">
        {error && (
          <div className="notice" role="alert">
            {error}
          </div>
        )}
        {!items.length && !error && (
          <div className="empty-state">
            <History size={28} />
            <h2>{loading ? 'Loading conversations…' : 'No conversations yet'}</h2>
            {!loading && <p>Send a message in Assistant to start your first conversation.</p>}
          </div>
        )}
        <div className="history-list">
          {items.map((item) => (
            <article className="history-row" key={item.id}>
              <button
                className="history-open"
                disabled={busy || loading}
                onClick={async () => {
                  setLoading(true)
                  if (!(await open(item.id))) {
                    setError('Could not open this conversation. Refresh and try again.')
                    setLoading(false)
                  }
                }}
              >
                <span className="tool-symbol">
                  <MessageSquare size={18} />
                </span>
                <span>
                  <strong>{item.title}</strong>
                  <small>
                    {new Date(item.updated_at).toLocaleString([], {
                      dateStyle: 'medium',
                      timeStyle: 'short',
                    })}
                  </small>
                </span>
                <ArrowRight size={16} />
              </button>
              {confirm === item.id ? (
                <div className="delete-actions">
                  <button className="secondary" disabled={loading} onClick={() => setConfirm('')}>
                    Keep
                  </button>
                  <button
                    className="danger"
                    disabled={loading || busy}
                    onClick={() => remove(item.id)}
                  >
                    Delete
                  </button>
                </div>
              ) : (
                <button
                  className="icon-button"
                  aria-label={'Delete ' + item.title}
                  disabled={busy || loading}
                  onClick={() => setConfirm(item.id)}
                >
                  <Trash2 size={17} />
                </button>
              )}
            </article>
          ))}
        </div>
        {more && (
          <button className="secondary" disabled={loading} onClick={() => load(items.length)}>
            Load more conversations
          </button>
        )}
      </div>
    </section>
  )
}
