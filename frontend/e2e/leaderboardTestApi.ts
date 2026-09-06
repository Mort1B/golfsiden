import { spawn, type ChildProcess } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { expect } from '@playwright/test'

// This opt-in suite owns this process, allowing a real transport interruption.
// Use only a freshly seeded disposable DATABASE_URL and keep port 3000 free.
export class LeaderboardTestApi {
  private child: ChildProcess | null = null

  async start() {
    if (!process.env.DATABASE_URL) throw new Error('A disposable DATABASE_URL is required')
    if (this.child) throw new Error('Test API is already running')
    const child = spawn(fileURLToPath(new URL('../../target/debug/golf-api', import.meta.url)), [], {
      env: { ...process.env, APP_ENV: 'development', SESSION_COOKIE_SECURE: 'false', CORS_ALLOWED_ORIGIN: 'http://127.0.0.1:5173', RUN_MIGRATIONS: 'false', PORT: '3000' }, stdio: 'ignore',
    })
    this.child = child
    let failure: Error | null = null
    child.on('error', error => { failure = error })
    await expect.poll(async () => {
      if (failure) throw failure
      if (child.exitCode !== null) throw new Error(`Test API exited (${child.exitCode})`)
      try { return (await fetch('http://127.0.0.1:3000/api/ready')).ok } catch { return false }
    }).toBe(true)
  }

  async stop() {
    const child = this.child
    this.child = null
    if (!child || child.exitCode !== null || child.signalCode !== null) return
    await new Promise<void>(resolve => {
      child.once('exit', () => resolve())
      // A crash closes existing SSE sockets immediately; no shared service is killed.
      child.kill('SIGKILL')
    })
  }
}
