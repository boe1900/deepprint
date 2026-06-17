import { defineConfig } from 'vitest/config'
import { devtools } from '@tanstack/devtools-vite'

import { tanstackRouter } from '@tanstack/router-plugin/vite'

import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

const devApiTarget =
  process.env.DEEPPRINT_DEV_API_TARGET ??
  `http://127.0.0.1:${process.env.DEEPPRINT_AGENT_PORT ?? process.env.DEEPPRINT_SERVER_PORT ?? '17801'}`

const config = defineConfig({
  resolve: {
    dedupe: ['react', 'react-dom', '@tanstack/react-router'],
    tsconfigPaths: true,
  },
  server: {
    proxy: {
      '/v1': {
        target: devApiTarget,
        changeOrigin: true,
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: "./src/test/setup.ts",
  },
  plugins: [
    devtools(),
    tailwindcss(),
    tanstackRouter({ target: 'react', autoCodeSplitting: true }),
    viteReact(),
  ],
})

export default config
