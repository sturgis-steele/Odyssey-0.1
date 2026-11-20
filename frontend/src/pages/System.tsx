import { useEffect, useState } from 'react'
import { RotateCw, Server, Wrench } from 'lucide-react'
import type { System as SystemState } from '../hooks/useSystem'
import { api, readable } from '../services/api'
import { toolName } from '../components/toolNames'

export function SystemPage({ system }: { system: SystemState }) {
  const { status, checked } = system
  const [names, setNames] = useState<string[]>([])
  const [error, setError] = useState('')
  useEffect(() => {
    let mounted = true
    if (status)
      api
        .tools()
        .then((tools) => {
          if (mounted) {
            setNames(tools.map((t) => toolName(t.name)))
            setError('')
          }
        })
        .catch((e) => {
          if (mounted) setError(readable(e))
        })
    return () => {
      mounted = false
    }
  }, [status])
  return (
    <section className="page secondary-page">
      <header className="page-heading">
        <h1>System</h1>
        <button className="secondary" disabled={system.checking} onClick={system.refresh}>
          <RotateCw size={16} />
          Refresh status
        </button>
      </header>
      <div className="page-content">
        <div className="system-panel">
          <h2>
            <Server size={19} />
            System information
          </h2>
          <dl>
            {Object.entries({
              'Backend connection': status ? 'Online' : checked ? 'Offline' : 'Checking',
              'Backend version': status?.version,
              'Inference provider': 'llama.cpp',
              'Configured model': status?.model,
              'Model connection': status?.model_server || 'Unknown',
              'Context budget': status && status.context_tokens + ' tokens',
              'Maximum output': status && status.max_output_tokens + ' tokens',
            }).map(([key, value]) => (
              <div key={key}>
                <dt>{key}</dt>
                <dd>{value || 'Unavailable'}</dd>
              </div>
            ))}
          </dl>
          {status && <p className="system-note">{status.diagnostic}</p>}
        </div>
        <div className="system-panel">
          <h2>
            <Wrench size={18} />
            Available tools
          </h2>
          <p>
            {!status ? 'Backend unavailable.' : error || names.join(' · ') || 'Checking tools…'}
          </p>
        </div>
      </div>
    </section>
  )
}
