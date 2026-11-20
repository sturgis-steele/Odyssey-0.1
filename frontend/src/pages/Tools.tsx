import { useEffect, useState } from 'react'
import { Play, RotateCw } from 'lucide-react'
import { api, readable } from '../services/api'
import { toolIcon, toolName } from '../components/toolNames'
import type { Outcome, ToolInfo } from '../types'

function ToolPanel({ tool }: { tool: ToolInfo }) {
  const [input, setInput] = useState('')
  const [result, setResult] = useState<Outcome | null>(null)
  const [running, setRunning] = useState(false)
  const calc = tool.name === 'calculator'
  const Icon = toolIcon(tool.name)
  async function run() {
    setRunning(true)
    try {
      setResult(await api.run(tool.name, calc ? { expression: input } : {}))
    } catch (err) {
      setResult({ ok: false, result: { error: readable(err) } })
    } finally {
      setRunning(false)
    }
  }
  return (
    <article className="tool-panel">
      <div className="tool-panel-title">
        <span className="tool-symbol">
          <Icon size={20} />
        </span>
        <h2>{toolName(tool.name)}</h2>
      </div>
      <p>{tool.description}</p>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          void run()
        }}
      >
        {calc && (
          <label>
            Expression
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              maxLength={192}
              placeholder="(125 / 5) + 17"
            />
          </label>
        )}
        <button className="secondary" disabled={running || (calc && !input.trim())}>
          <Play size={14} />
          {running ? 'Running…' : 'Run tool'}
        </button>
      </form>
      {result && (
        <div role="status" className={result.ok ? 'tool-output' : 'notice'}>
          {result.ok ? (
            <dl>
              {Object.entries(result.result).map(([key, value]) => (
                <div key={key}>
                  <dt>{key.replaceAll('_', ' ')}</dt>
                  <dd>{String(value)}</dd>
                </div>
              ))}
            </dl>
          ) : (
            String(result.result.error)
          )}
        </div>
      )}
    </article>
  )
}

export function Tools({ online }: { online: boolean }) {
  const [tools, setTools] = useState<ToolInfo[]>([])
  const [error, setError] = useState('')
  const [revision, setRevision] = useState(0)
  useEffect(() => {
    let current = true
    api
      .tools()
      .then((v) => {
        if (current) {
          setTools(v)
          setError('')
        }
      })
      .catch((e) => {
        if (current) setError(readable(e))
      })
    return () => {
      current = false
    }
  }, [online, revision])
  return (
    <section className="page secondary-page">
      <header className="page-heading">
        <h1>Tools</h1>
      </header>
      <div className="page-content">
        <p className="intro">Run a device tool directly, even when the model is offline.</p>
        {error && (
          <div className="notice" role="alert">
            {error}
            <button className="text-button" onClick={() => setRevision((v) => v + 1)}>
              <RotateCw size={14} />
              Retry
            </button>
          </div>
        )}
        <div className="tools-grid">
          {tools.map((tool) => (
            <ToolPanel key={tool.name} tool={tool} />
          ))}
        </div>
      </div>
    </section>
  )
}
