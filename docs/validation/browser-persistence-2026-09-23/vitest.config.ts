import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
export default defineConfig({root:'/tmp/golf-persistence-assessment',plugins:[react()],test:{environment:'jsdom',include:['cache.probe.test.tsx']}})
