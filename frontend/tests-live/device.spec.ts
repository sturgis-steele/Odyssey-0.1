import { expect, test } from '@playwright/test'

// These are opt-in tests with real local storage and model processes.
test('real GGUF calculation and saved transcript with only loopback traffic', async ({
  page,
  context,
}, testInfo) => {
  test.skip(process.env.ODYSSEY_MODEL_STOPPED === '1', 'Run when llama-server is online.')
  const remote: string[] = []
  await context.route('**/*', (route) => {
    const url = new URL(route.request().url())
    if (url.hostname === '127.0.0.1') return route.continue()
    remote.push(url.hostname)
    return route.abort()
  })
  await page.goto('/')
  await expect(page.locator('.connections')).toContainText('ModelOnline')
  await page.evaluate(() => document.fonts.ready)
  await page.screenshot({ path: testInfo.outputPath('assistant-desktop.png') })
  for (const [width, height] of [
    [1024, 600],
    [800, 480],
  ]) {
    await page.setViewportSize({ width, height })
    await expect(page.getByRole('textbox')).toBeInViewport()
    await page.screenshot({ path: testInfo.outputPath('assistant-' + width + '.png') })
  }
  await page.getByRole('button', { name: /Run a calculation/ }).click()
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(page.locator('.activity summary')).toContainText('Calculator', { timeout: 120000 })
  await expect(page.locator('.activity summary')).toContainText('Completed')
  await expect(page.locator('.markdown')).toContainText(/38,?151/, { timeout: 120000 })
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toHaveCount(0, {
    timeout: 120000,
  })
  await page.reload()
  await page.getByRole('button', { name: 'Conversations', exact: true }).click()
  await page
    .getByRole('button', { name: /^What is 157 multiplied by 243/ })
    .first()
    .click()
  await expect(page.locator('.markdown')).toContainText(/38,?151/)
  await page.getByRole('textbox').fill('What result did you just give me? Reply with the number.')
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(page.locator('.exchange').nth(1).locator('.markdown')).toContainText(/38,?151/, {
    timeout: 120000,
  })
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toHaveCount(0, {
    timeout: 120000,
  })
  expect(remote).toEqual([])
})

test('stopping generation releases the slot and saves its terminal state', async ({
  page,
  request,
}) => {
  test.skip(process.env.ODYSSEY_MODEL_STOPPED === '1', 'Run when llama-server is online.')
  await page.goto('/')
  await page
    .getByRole('textbox')
    .fill('Write a long story about an expedition across the mountains.')
  const start = page.waitForResponse((r) => r.url().endsWith('/api/chat/stream'))
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  const res = await start
  const id = res.request().postDataJSON().conversation_id
  const overlap = await request.post('/api/chat/stream', {
    data: { conversation_id: id, message: 'Another message' },
  })
  expect(overlap.status()).toBe(409)
  expect((await request.delete('/api/conversations/' + id)).status()).toBe(409)
  await page.getByRole('button', { name: 'Stop', exact: true }).click()
  await expect
    .poll(
      async () => (await (await request.get('/api/conversations/' + id)).json()).turns[0]?.status,
      { timeout: 10000 },
    )
    .toBe('interrupted')
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toHaveCount(0)
  await page.getByRole('textbox').fill('Hello.')
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(page.locator('.exchange').last().locator('.markdown')).toContainText(/\w/, {
    timeout: 120000,
  })
  await expect(page.getByRole('button', { name: 'Stop', exact: true })).toHaveCount(0, {
    timeout: 120000,
  })
})

test('stopped llama-server leaves the backend, saved history and tools usable', async ({
  page,
}) => {
  test.skip(
    process.env.ODYSSEY_MODEL_STOPPED !== '1',
    'Stop only the test model process and set ODYSSEY_MODEL_STOPPED=1.',
  )
  await page.goto('/')
  await expect(page.locator('.notice')).toContainText('Model offline', { timeout: 15000 })
  await expect(page.locator('.connections')).toContainText('BackendOnline')
  await page.getByRole('button', { name: 'Conversations', exact: true }).click()
  await expect(page.locator('.history-open').first()).toBeVisible()
  await page.getByRole('button', { name: 'Tools', exact: true }).click()
  await page.getByLabel('Expression').fill('2 ** 10')
  await page.getByRole('button', { name: 'Run tool' }).first().click()
  await expect(page.locator('.tool-output')).toContainText('1024')
})
