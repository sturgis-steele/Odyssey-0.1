import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, '../backend', 'BACKEND_')
  const host = process.env.BACKEND_HOST || env.BACKEND_HOST || '127.0.0.1'
  if (!['127.0.0.1', 'localhost', '::1'].includes(host))
    throw new Error('Use a loopback backend host.')
  const target =
    'http://' +
    (host === '::1' ? '[::1]' : host) +
    ':' +
    (process.env.BACKEND_PORT || env.BACKEND_PORT || '8000')
  return {
    plugins: [react()],
    server: {
      host: '127.0.0.1',
      port: 5173,
      strictPort: true,
      proxy: { '/api': target },
    },
    build: { sourcemap: false, license: { fileName: 'THIRD-PARTY-NOTICES.md' } },
  }
})
