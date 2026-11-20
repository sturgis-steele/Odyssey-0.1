import type { ChatEvent, Detail, Outcome, Status, Summary, ToolInfo } from '../types'

export function readable(error: unknown) {
  return error instanceof Error ? error.message : 'The request could not be completed.'
}

async function response(path: string, init: RequestInit = {}) {
  let res: Response
  try {
    res = await fetch('/api/' + path, { cache: 'no-store', ...init })
  } catch (err) {
    if (init.signal?.aborted) throw err
    throw new Error('Cannot reach the local backend. Check the connection and retry.')
  }
  if (!res.ok) {
    const problem = await res.json().catch(() => ({}))
    throw new Error(
      typeof problem.detail === 'string' ? problem.detail : 'The local backend is unavailable.',
    )
  }
  return res
}

async function get<T>(path: string, init: RequestInit = {}): Promise<T> {
  return (await response(path, { signal: AbortSignal.timeout(10000), ...init })).json()
}

function post(data: object): RequestInit {
  return {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  }
}

export const api = {
  status: () => get<Status>('system/status'),
  tools: () => get<ToolInfo[]>('tools'),
  run: (name: string, args: object) =>
    get<Outcome>('tools/' + encodeURIComponent(name) + '/run', post({ arguments: args })),
  create: () => get<Summary>('conversations', { method: 'POST' }),
  list: (offset = 0) => get<Summary[]>('conversations?limit=30&offset=' + offset),
  detail: (id: string, before?: number) =>
    get<Detail>('conversations/' + id + '?limit=20' + (before ? '&before=' + before : '')),
  remove: (id: string) =>
    response('conversations/' + id, { method: 'DELETE', signal: AbortSignal.timeout(10000) }),
  chat: async (
    id: string,
    message: string,
    signal: AbortSignal,
    receive: (event: ChatEvent) => void,
  ) => {
    const res = await response('chat/stream', { ...post({ conversation_id: id, message }), signal })
    if (!res.body || !res.headers.get('content-type')?.includes('text/event-stream')) {
      throw new Error('The backend returned an invalid chat stream.')
    }
    await readStream(res.body, receive)
  },
}

function decodeEvent(block: string): ChatEvent | undefined {
  const lines = block.split(/\r?\n/)
  const type = lines
    .find((line) => line.startsWith('event:'))
    ?.slice(6)
    .trim()
  if (!type && lines.every((line) => !line || line.startsWith(':'))) return
  const raw = lines
    .filter((line) => line.startsWith('data:'))
    .map((line) => line.slice(5).trimStart())
    .join('\n')
  const data = JSON.parse(raw)
  const has = (key: string) => typeof data?.[key] === 'string'
  const valid =
    (type === 'message_start' && has('turn_id') && has('conversation_id')) ||
    (type === 'text_delta' && has('text')) ||
    (type === 'tool_start' && has('call_id') && has('name')) ||
    (type === 'tool_result' &&
      has('call_id') &&
      has('name') &&
      typeof data.ok === 'boolean' &&
      data.result != null) ||
    (type === 'message_complete' && has('turn_id')) ||
    (type === 'error' && has('code') && has('message'))
  if (!valid) throw new Error('Invalid event.')
  return { ...data, type } as ChatEvent
}

export async function readStream(
  body: ReadableStream<Uint8Array>,
  receive: (event: ChatEvent) => void,
) {
  const reader = body.getReader()
  const decoder = new TextDecoder()
  let buffer = '',
    started = false,
    terminal = false
  try {
    for (;;) {
      const { value, done } = await reader.read()
      buffer += decoder.decode(value, { stream: !done })
      let boundary: RegExpExecArray | null
      while ((boundary = /\r?\n\r?\n/.exec(buffer))) {
        const block = buffer.slice(0, boundary.index)
        buffer = buffer.slice(boundary.index + boundary[0].length)
        let event: ChatEvent | undefined
        try {
          event = decodeEvent(block)
        } catch {
          throw new Error('The backend returned an invalid chat event.')
        }
        if (!event) continue
        if (
          terminal ||
          (!started && event.type !== 'message_start') ||
          (started && event.type === 'message_start')
        ) {
          throw new Error('The chat stream arrived out of order.')
        }
        started = true
        terminal = event.type === 'message_complete' || event.type === 'error'
        receive(event)
      }
      if (done) break
    }
    if (!terminal || buffer.trim())
      throw new Error('The response was interrupted. Reopen the conversation to see saved text.')
  } finally {
    await reader.cancel().catch(() => {})
    reader.releaseLock()
  }
}
