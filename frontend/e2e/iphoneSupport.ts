import { expect, type Page, type TestInfo } from '@playwright/test'

export const iphoneViewports = [
  [320, 600], [390, 844], [393, 852], [402, 874], [430, 932], [440, 956],
  [844, 390], [852, 393], [874, 402], [932, 430], [956, 440], [852, 260], [1280, 900],
] as const

export async function layout(page: Page, info: TestInfo, state: string, selector: string) {
  for (const [width, height] of iphoneViewports) {
    await page.setViewportSize({ width, height })
    const region = page.locator(selector)
    await expect(region).toBeVisible()
    expect(await page.evaluate(() => document.documentElement.scrollWidth), `${state} ${width}x${height}`).toBeLessThanOrEqual(width)
    for (const control of await region.locator('button:visible, select:visible, summary:visible').all()) {
      const box = await control.boundingBox()
      expect(box?.height, `${state}: ${await control.textContent()}`).toBeGreaterThanOrEqual(44)
      expect(box?.width).toBeGreaterThanOrEqual(44)
    }
    await page.evaluate(() => scrollTo(0, 0))
    await page.screenshot({ path: `/tmp/iphone-${info.project.name}-${state}-${width}-${height}.png` })
  }
}

export async function reachable(page: Page, selector: string) {
  const control = page.locator(selector).last()
  // Model deliberate scrolling as well as focus; keyboard viewport movement is device acceptance.
  await control.evaluate(el => el.scrollIntoView({ block: 'center' }))
  await control.focus()
  await expect(control).toBeFocused()
  await expect.poll(() => control.evaluate(el => {
    const box = el.getBoundingClientRect()
    const hit = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2)
    return box.top >= 0 && box.bottom <= innerHeight && (hit === el || el.contains(hit))
  })).toBe(true)
}

export function observe(page: Page) {
  const errors: string[] = []
  const expected = new Map<string, number>()
  page.on('pageerror', error => errors.push(error.message))
  page.on('console', message => {
    if (message.type() === 'error' && !/Failed to load resource/.test(message.text())) errors.push(message.text())
  })
  page.on('response', response => {
    if (response.status() >= 400 && expected.get(new URL(response.url()).pathname) !== response.status()) errors.push(`${response.status()} ${new URL(response.url()).pathname}`)
  })
  page.on('requestfailed', request => {
    const failure = request.failure()?.errorText
    // Navigation cancels GETs and SSE in both engines. All write failures stay visible.
    if (request.method() === 'GET' && (failure === 'net::ERR_ABORTED' || failure === 'Load request cancelled')) return
    errors.push(`${new URL(request.url()).pathname}: ${failure}`)
  })
  return { expected, check: () => expect(errors).toEqual([]) }
}
