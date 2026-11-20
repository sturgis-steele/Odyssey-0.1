import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  globalSetup: './tests/start-vite.ts',
  workers: 2,
  use: {
    baseURL: 'http://127.0.0.1:4176',
    viewport: { width: 1280, height: 800 },
    launchOptions: { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || undefined },
  },
})
