import { defineConfig } from '@playwright/test'

const port = Number(process.env.ODYSSEY_LIVE_PORT || 8000)
if (!Number.isInteger(port) || port < 1 || port > 65535)
  throw new Error('Invalid live backend port.')

export default defineConfig({
  testDir: './tests-live',
  workers: 1,
  timeout: 180000,
  use: {
    baseURL: 'http://127.0.0.1:' + port,
    viewport: { width: 1280, height: 800 },
    launchOptions: { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || undefined },
  },
})
