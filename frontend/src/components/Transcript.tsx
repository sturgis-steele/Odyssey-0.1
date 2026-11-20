import { memo, useEffect, useRef } from 'react'
import Markdown from 'react-markdown'
import { Check, ChevronDown, CircleAlert, LoaderCircle, Terminal, UserRound } from 'lucide-react'
import type { Turn } from '../types'
import type { Chat } from '../hooks/useChat'
import { toolIcon, toolName } from './toolNames'

const stamp = (date: string) =>
  new Date(date).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })

const Exchange = memo(function Exchange({ turn }: { turn: Turn }) {
  return (
    <article className="exchange">
      <div className="message-label">
        <UserRound size={15} /> You <time>{stamp(turn.created_at)}</time>
      </div>
      <div className="user-message">{turn.user}</div>
      <div className="message-label assistant-label">
        <Terminal size={16} /> Odyssey
      </div>
      {turn.tools.map((tool) => {
        const Icon = toolIcon(tool.name)
        const State = tool.ok === null ? LoaderCircle : tool.ok ? Check : CircleAlert
        return (
          <details className="activity" key={tool.call_id}>
            <summary>
              <Icon size={15} />
              <span>{toolName(tool.name)}</span>
              <span className="activity-state">
                <State size={13} />
                {tool.ok === null ? 'Running' : tool.ok ? 'Completed' : 'Rejected'}
              </span>
              <ChevronDown size={13} />
            </summary>
            <pre>{JSON.stringify(tool.result, null, 2)}</pre>
          </details>
        )
      })}
      {turn.assistant && (
        <div className="markdown">
          <Markdown skipHtml disallowedElements={['img', 'a']} unwrapDisallowed>
            {turn.assistant}
          </Markdown>
        </div>
      )}
      {turn.status === 'generating' && (
        <span role="status" className="generating">
          <i />
          Generating response…
        </span>
      )}
      {turn.error && (
        <p className="turn-error" role="status">
          <CircleAlert size={15} />
          {turn.error}
        </p>
      )}
    </article>
  )
})

export function Transcript({ chat }: { chat: Chat }) {
  const scroller = useRef<HTMLDivElement>(null)
  const follow = useRef(true)
  const prevHeight = useRef(0)
  const prevFirst = useRef<string | undefined>(undefined)
  useEffect(() => {
    const el = scroller.current
    if (!el) return
    if (
      prevHeight.current &&
      prevFirst.current !== chat.turns[0]?.id &&
      prevFirst.current &&
      chat.turns.some((t) => t.id === prevFirst.current)
    ) {
      el.scrollTop += el.scrollHeight - prevHeight.current
    } else if (follow.current) el.scrollTop = el.scrollHeight
    prevHeight.current = el.scrollHeight
    prevFirst.current = chat.turns[0]?.id
  }, [chat.turns])
  return (
    <div
      ref={scroller}
      className="transcript"
      aria-label="Conversation messages"
      onScroll={(e) => {
        const el = e.currentTarget
        follow.current = el.scrollHeight - el.clientHeight - el.scrollTop < 64
      }}
    >
      {chat.before && (
        <button className="secondary" disabled={chat.loading || chat.busy} onClick={chat.older}>
          Load earlier messages
        </button>
      )}
      {chat.turns.map((turn) => (
        <Exchange key={turn.id} turn={turn} />
      ))}
    </div>
  )
}
