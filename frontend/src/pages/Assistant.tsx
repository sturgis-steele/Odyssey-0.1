import { useRef } from 'react'
import {
  ArrowUp,
  ArrowUpRight,
  CircleAlert,
  CircuitBoard,
  CornerDownLeft,
  LockKeyhole,
  MessageSquarePlus,
  Radio,
  Square,
} from 'lucide-react'
import type { Chat } from '../hooks/useChat'
import type { System } from '../hooks/useSystem'
import { prompts } from '../components/toolNames'
import { Transcript } from '../components/Transcript'

export function Assistant({ chat, system }: { chat: Chat; system: System }) {
  const input = useRef<HTMLTextAreaElement>(null)
  const { status, checked } = system
  const ready = status?.model_server === 'online'
  const locked = chat.busy || chat.loading || chat.watching
  const unavailable = !checked
    ? 'Checking connections'
    : !status
      ? 'Backend offline'
      : status.model_server === 'loading'
        ? 'Model loading'
        : status.model_server === 'misconfigured'
          ? 'Model alias unavailable'
          : 'Model offline'
  return (
    <section className="assistant page">
      <header className="page-heading">
        <div>
          {chat.conversation && <p className="conversation-title">{chat.conversation.title}</p>}
          <h1>Assistant</h1>
        </div>
        <button
          className="secondary new-chat"
          disabled={chat.busy}
          onClick={() => {
            chat.fresh()
            input.current?.focus()
          }}
        >
          <MessageSquarePlus size={17} />
          <span>New conversation</span>
        </button>
      </header>
      {chat.turns.length ? (
        <Transcript key={chat.conversation?.id} chat={chat} />
      ) : (
        <div className="welcome-scroll">
          <div className="welcome">
            <div className="welcome-copy">
              <div className="hero-label">
                <Radio size={15} /> On-device assistant
              </div>
              <h2>How can I help?</h2>
              <p>Ask a question or put a local tool to work.</p>
            </div>
          </div>
          <section className="suggestions" aria-labelledby="prompt-title">
            <h3 className="section-label" id="prompt-title">
              Quick prompts
            </h3>
            <div className="prompt-grid">
              {prompts.map(({ icon: Icon, name, title, prompt }) => (
                <button
                  key={name}
                  className="prompt-card"
                  disabled={locked}
                  onClick={() => {
                    chat.setDraft(prompt)
                    input.current?.focus()
                  }}
                >
                  <span className="card-icons">
                    <span className="tool-symbol">
                      <Icon size={20} />
                    </span>
                    <ArrowUpRight size={16} />
                  </span>
                  <strong>{title}</strong>
                  <span className="prompt-copy">{prompt}</span>
                </button>
              ))}
            </div>
          </section>
        </div>
      )}
      <div className="compose-area">
        {!ready && checked && (
          <div className="notice">
            <CircleAlert size={17} />
            <span>
              <strong>{unavailable}.</strong>{' '}
              {status ? status.diagnostic : 'Start the Odyssey backend to reconnect.'}
            </span>
            <button className="text-button" onClick={system.refresh} disabled={system.checking}>
              Retry
            </button>
          </div>
        )}
        {chat.error && (
          <div className="notice" role="alert">
            <CircleAlert size={16} />
            {chat.error}
          </div>
        )}
        <form
          className="composer"
          onSubmit={(e) => {
            e.preventDefault()
            if (ready && !locked) void chat.send()
          }}
        >
          <textarea
            ref={input}
            rows={2}
            maxLength={8192}
            value={chat.draft}
            disabled={chat.busy || chat.watching}
            aria-label="Message Odyssey"
            placeholder="Message Odyssey…"
            onChange={(e) => chat.setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (
                e.key === 'Enter' &&
                !e.shiftKey &&
                !e.nativeEvent.isComposing &&
                !window.matchMedia('(pointer: coarse)').matches
              ) {
                e.preventDefault()
                if (ready && !locked) void chat.send()
              }
            }}
          />
          <div className="composer-actions">
            <span className="model-chip">
              <CircuitBoard size={14} />
              {status?.model || 'Local model'}
            </span>
            {chat.busy ? (
              <button type="button" className="send" onClick={chat.stop}>
                <Square size={14} />
                Stop
              </button>
            ) : (
              <button
                type="submit"
                className="send"
                disabled={!ready || locked || !chat.draft.trim()}
              >
                <ArrowUp size={17} />
                Send
              </button>
            )}
          </div>
        </form>
        <div className="composer-meta">
          <span>
            <LockKeyhole size={12} /> Conversations stay on this device
          </span>
          <span className="key-hint">
            <CornerDownLeft size={13} /> Enter to send
          </span>
        </div>
      </div>
    </section>
  )
}
