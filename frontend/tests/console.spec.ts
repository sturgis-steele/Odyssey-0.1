import { expect, test, type Page } from '@playwright/test'
import type { Detail, ModelState, Turn } from '../src/types'

const id = '00000000-0000-4000-8000-000000000001'
const stamp = '2026-11-20T12:00:00Z'
const summary = { id, title: 'Earlier question', created_at: stamp, updated_at: stamp }
const historyTurn: Turn = {
  id: 'saved',
  seq: 1,
  user: 'Earlier question',
  assistant: 'A **saved** response.\n\n```py\nprint(42)\n```',
  status: 'complete',
  error: null,
  created_at: stamp,
  tools: [],
}
const saved: Detail = { ...summary, turns: [historyTurn], next_before: null }

async function fixture(page: Page, state: ModelState | 'disconnected' = 'online') {
  await page.route('**/api/**', async (route) => {
    const req = route.request(),
      path = new URL(req.url()).pathname
    if (state === 'disconnected')
      return route.fulfill({ status: 503, json: { detail: 'Backend unavailable.' } })
    if (path.endsWith('/system/status'))
      return route.fulfill({
        json: {
          backend: 'online',
          model_server: state,
          status: state === 'online' ? 'ready' : 'degraded',
          diagnostic: state === 'online' ? 'Local inference is ready.' : 'Check llama-server.',
          model: 'test-local',
          version: '0.1.0',
          context_tokens: 4096,
          max_output_tokens: 512,
        },
      })
    if (path.endsWith('/conversations'))
      return route.fulfill({ json: req.method() === 'POST' ? summary : [summary] })
    if (path.endsWith('/conversations/' + id)) return route.fulfill({ json: saved })
    if (path.endsWith('/tools'))
      return route.fulfill({
        json: [
          {
            name: 'calculator',
            description: 'Calculate an expression.',
            permission: 'read-only',
            parameters: {},
          },
        ],
      })
    if (path.endsWith('/calculator/run'))
      return route.fulfill({ json: { ok: true, result: { value: 42 } } })
    return route.fulfill({ status: 404, json: {} })
  })
  await page.goto('/')
}

test('prompt, chat input, progressive result, tool activity and safe Markdown', async ({
  page,
}) => {
  // Fragment both field names and a multi-byte character across transport boundaries.
  await page.addInitScript(() => {
    const original = window.fetch
    window.fetch = async (input, init) => {
      if (!String(input).endsWith('/chat/stream')) return original(input, init)
      const body = JSON.parse(String(init?.body))
      if (body.message !== 'What is 157 multiplied by 243?') throw new Error('Unexpected prompt')
      const events = [
        ['message_start', { turn_id: 'live', conversation_id: body.conversation_id }],
        ['tool_start', { call_id: 'a', name: 'calculator' }],
        ['tool_result', { call_id: 'a', name: 'calculator', ok: true, result: { value: 38151 } }],
        ['text_delta', { text: '**38,151** café' }],
        ['text_delta', { text: ' <img src="https://example.invalid/pixel">' }],
        ['message_complete', { turn_id: 'live' }],
      ]
      const encoder = new TextEncoder()
      const stream = new ReadableStream({
        async start(controller) {
          for (const [name, data] of events) {
            const bytes = encoder.encode(
              'event: ' + name + '\r\ndata: ' + JSON.stringify(data) + '\r\n\r\n',
            )
            for (const byte of bytes) controller.enqueue(Uint8Array.of(byte))
            await new Promise((resolve) => setTimeout(resolve, 140))
          }
          controller.close()
        },
      })
      return new Response(stream, { headers: { 'Content-Type': 'text/event-stream' } })
    }
  })
  await fixture(page)
  await page.route('**/api/conversations/' + id + '?**', (route) =>
    route.fulfill({
      json: {
        ...saved,
        turns: [
          {
            ...historyTurn,
            id: 'live',
            assistant: '**38,151** café',
            user: 'What is 157 multiplied by 243?',
            tools: [{ call_id: 'a', name: 'calculator', ok: true, result: { value: 38151 } }],
          },
        ],
      },
    }),
  )
  const input = page.getByRole('textbox', { name: 'Message Odyssey' })
  await expect(page.getByRole('button', { name: 'Send', exact: true })).toBeDisabled()
  await page.getByRole('button', { name: /Run a calculation/ }).click()
  await expect(input).toHaveValue('What is 157 multiplied by 243?')
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toBeVisible()
  await expect(page.locator('.activity summary')).toContainText('Completed')
  await expect(page.locator('.markdown strong')).toHaveText('38,151')
  await expect(page.locator('.markdown')).toContainText('café')
  await expect(page.locator('.markdown img, .markdown script')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toHaveCount(0)
  await page.locator('.activity summary').click()
  await expect(page.locator('.activity pre')).toContainText('38151')
  await page.getByRole('button', { name: 'New conversation' }).click()
  await expect(page.getByRole('heading', { name: 'How can I help?' })).toBeVisible()
})

test('distinguishes unavailable backend', async ({ page }) => {
  await fixture(page, 'disconnected')
  await expect(page.locator('.notice')).toContainText('Backend offline')
  await page.getByRole('textbox').fill('Draft remains editable')
  await expect(page.getByRole('button', { name: 'Send', exact: true })).toBeDisabled()
  await page.getByRole('button', { name: 'System', exact: true }).click()
  await expect(page.locator('dd').first()).toHaveText('Offline')
})

test('model offline keeps saved chats and tools usable', async ({ page }) => {
  await fixture(page, 'offline')
  await expect(page.locator('.notice')).toContainText('Model offline')
  await page.getByRole('button', { name: 'Conversations', exact: true }).click()
  await page.getByRole('button', { name: /^Earlier question/ }).click()
  await expect(page.locator('.markdown strong')).toHaveText('saved')
  await expect(page.locator('.markdown pre code')).toHaveText(/print\(42\)/)
  await page.getByRole('button', { name: 'Tools', exact: true }).click()
  await page.getByLabel('Expression').fill('7 * 6')
  await page.getByRole('button', { name: 'Run tool' }).click()
  await expect(page.locator('.tool-output')).toContainText('42')
})

test('an open conversation recovers after a temporary history read failure', async ({ page }) => {
  await fixture(page)
  let reads = 0
  await page.route('**/api/conversations/' + id + '?**', async (route) => {
    reads++
    if (reads === 2) return route.fulfill({ status: 503, json: { detail: 'Storage is busy.' } })
    const turn = reads === 1 ? { ...historyTurn, assistant: '', status: 'generating' } : historyTurn
    return route.fulfill({ json: { ...saved, turns: [turn] } })
  })
  await page.getByRole('button', { name: 'Conversations', exact: true }).click()
  await page.getByRole('button', { name: /^Earlier question/ }).click()
  await expect(page.getByRole('alert')).toHaveText('Storage is busy.')
  await expect(page.locator('.markdown strong')).toHaveText('saved', { timeout: 10000 })
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('deletion requires confirmation and updates the list', async ({ page }) => {
  await fixture(page)
  let deleted = false
  await page.route('**/api/conversations?**', (route) =>
    route.fulfill({ json: deleted ? [] : [summary] }),
  )
  await page.route('**/api/conversations/' + id, async (route) => {
    expect(route.request().method()).toBe('DELETE')
    deleted = true
    await route.fulfill({ status: 204 })
  })
  await page.getByRole('button', { name: 'Conversations', exact: true }).click()
  await page.getByRole('button', { name: 'Delete Earlier question' }).click()
  await page.getByRole('button', { name: 'Keep', exact: true }).click()
  expect(deleted).toBe(false)
  await page.getByRole('button', { name: 'Delete Earlier question' }).click()
  await page.getByRole('button', { name: 'Delete', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'No conversations yet' })).toBeVisible()
})

test('stream parser rejects missing terminal events and malformed payloads', async ({ page }) => {
  await fixture(page)
  const errors = await page.evaluate(async () => {
    const modulePath = '/src/services/api.ts'
    const { readStream } = await import(modulePath)
    const start = 'event: message_start\ndata: {"turn_id":"x","conversation_id":"c"}\n\n'
    const bad = 'event: text_delta\ndata: {"text":3}\n\n'
    const errors = []
    for (const data of [start, start + bad]) {
      try {
        await readStream(new Response(data).body, () => {})
      } catch (e) {
        errors.push((e as Error).message)
      }
    }
    return errors
  })
  expect(errors).toEqual([
    expect.stringContaining('interrupted'),
    expect.stringContaining('invalid chat event'),
  ])
})

for (const [width, height] of [
  [1024, 600],
  [800, 480],
  [390, 700],
]) {
  test('touch layout and fullscreen at ' + width + ' × ' + height, async ({ page }) => {
    await page.setViewportSize({ width, height })
    await fixture(page)
    await page.getByRole('button', { name: /Check local time/ }).click()
    await expect(page.getByRole('textbox')).toHaveValue('What is the current local date and time?')
    await expect(page.getByRole('button', { name: 'Send', exact: true })).toBeInViewport()
    const layout = await page.evaluate(() => ({
      fits: document.documentElement.scrollWidth <= innerWidth,
      target: document.querySelector('nav button')!.getBoundingClientRect().height,
    }))
    expect(layout.fits).toBe(true)
    expect(layout.target).toBeGreaterThanOrEqual(44)
    await page.getByRole('button', { name: 'Enter fullscreen' }).click()
    await expect(page.getByRole('button', { name: 'Exit fullscreen' })).toBeVisible()
  })
}
