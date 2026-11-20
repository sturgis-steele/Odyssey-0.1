import { useEffect, useState } from 'react'
import {
  ChevronRight,
  Clock3,
  Expand,
  History,
  MessageSquare,
  Minimize,
  SlidersHorizontal,
  SquareTerminal,
  Wrench,
} from 'lucide-react'
import { Assistant } from './pages/Assistant'
import { Conversations } from './pages/Conversations'
import { Tools } from './pages/Tools'
import { SystemPage } from './pages/System'
import { useSystem } from './hooks/useSystem'
import { useChat } from './hooks/useChat'
import type { Page } from './types'

const nav = [
  { name: 'Assistant', icon: MessageSquare },
  { name: 'Conversations', icon: History },
  { name: 'Tools', icon: Wrench },
  { name: 'System', icon: SlidersHorizontal },
] as const

export default function App() {
  const [page, setPage] = useState<Page>('Assistant')
  const [full, setFull] = useState(false)
  const [fullError, setFullError] = useState('')
  const [time, setTime] = useState(new Date())
  const system = useSystem()
  const chat = useChat(system.refresh)
  const { status, checked } = system
  useEffect(() => {
    document.title = 'Odyssey · ' + page
  }, [page])
  useEffect(() => {
    const changed = () => setFull(Boolean(document.fullscreenElement))
    document.addEventListener('fullscreenchange', changed)
    const timer = window.setInterval(() => setTime(new Date()), 15000)
    return () => {
      document.removeEventListener('fullscreenchange', changed)
      window.clearInterval(timer)
    }
  }, [])
  async function fullscreen() {
    try {
      setFullError('')
      if (document.fullscreenElement) await document.exitFullscreen()
      else await document.documentElement.requestFullscreen()
    } catch {
      setFullError('Fullscreen is unavailable in this browser window.')
    }
  }
  const modelState = status?.model_server || (checked ? 'unknown' : 'checking')
  const modelDisplay = {
    online: 'Online',
    offline: 'Offline',
    loading: 'Loading',
    misconfigured: 'Check alias',
    checking: 'Checking',
    unknown: 'Unknown',
  }[modelState]
  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <img src="/odyssey.svg" alt="" width="38" height="38" />
          <div>
            <strong>Odyssey</strong>
            <span>Field console</span>
          </div>
        </div>
        <nav aria-label="Main navigation">
          {nav.map(({ name, icon: Icon }) => (
            <button
              key={name}
              aria-current={page === name ? 'page' : undefined}
              aria-label={name}
              className={page === name ? 'active' : ''}
              onClick={() => setPage(name)}
            >
              <Icon size={19} />
              <span>{name}</span>
            </button>
          ))}
        </nav>
        <div className="connections">
          <h2 className="section-label">Device status</h2>
          <div>
            <span>
              <i className={'dot ' + (status ? 'good' : checked ? 'bad' : '')} />
              Backend
            </span>
            <small>{status ? 'Online' : checked ? 'Offline' : 'Checking'}</small>
          </div>
          <div>
            <span>
              <i
                className={
                  'dot ' +
                  (modelState === 'online' ? 'good' : modelState === 'unknown' ? '' : 'warn')
                }
              />
              Model
            </span>
            <small>{modelDisplay}</small>
          </div>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <div className="crumb">
            <SquareTerminal size={17} />
            <span>Console</span>
            <ChevronRight size={13} />
            <strong>{page}</strong>
          </div>
          <div className="top-actions">
            <button className="mobile-status" onClick={() => setPage('System')}>
              <i className={'dot ' + (status?.model_server === 'online' ? 'good' : 'warn')} />
              {!checked
                ? 'Checking'
                : !status
                  ? 'Backend offline'
                  : status.model_server === 'online'
                    ? 'Device ready'
                    : 'Model ' + modelDisplay.toLowerCase()}
            </button>
            <time>
              <Clock3 size={14} />
              {time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
            </time>
            <button
              className="icon-button"
              aria-label={full ? 'Exit fullscreen' : 'Enter fullscreen'}
              title={full ? 'Exit fullscreen' : 'Enter fullscreen'}
              onClick={fullscreen}
            >
              {full ? <Minimize size={18} /> : <Expand size={18} />}
            </button>
          </div>
        </header>
        {fullError && (
          <div className="notice" role="alert">
            {fullError}
            <button className="text-button" onClick={() => setFullError('')}>
              Dismiss
            </button>
          </div>
        )}
        {page === 'Assistant' && <Assistant chat={chat} system={system} />}
        {page === 'Conversations' && (
          <Conversations
            online={Boolean(status)}
            busy={chat.busy}
            removed={(id) => {
              if (id === chat.conversation?.id) chat.fresh()
            }}
            open={async (id) => {
              const loaded = await chat.open(id)
              if (loaded) setPage('Assistant')
              return loaded
            }}
          />
        )}
        {page === 'Tools' && <Tools online={Boolean(status)} />}
        {page === 'System' && <SystemPage system={system} />}
      </main>
    </div>
  )
}
