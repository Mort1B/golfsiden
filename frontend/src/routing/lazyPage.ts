import { createElement, useEffect, useState, type ReactNode } from 'react'
import { RouteLoadError, RouteLoading } from '../ui/RouteLoadState'

export function lazyPage<Props extends object>(load: () => Promise<(props: Props) => ReactNode>) {
  type PageComponent = (props: Props) => ReactNode
  // Cache code only, never page props, account data or rendered elements. An
  // explicit loader avoids Suspense's fallback delay on warm full navigations.
  let loaded: PageComponent | undefined
  let pending: Promise<PageComponent> | undefined
  return function DeferredPage(props: Props) {
    const [Page, setPage] = useState(() => loaded)
    const [failed, setFailed] = useState(false)
    useEffect(() => {
      let active = true
      pending ??= Promise.resolve().then(load).then(component => { loaded = component; return component })
      void pending.then(component => { if (active) setPage(() => component) }, () => { if (active) setFailed(true) })
      return () => { active = false }
    }, [])
    // Failed native imports/preloads may stay cached for this document. Only an
    // explicit guarded reload retries. Rendering errors follow existing route
    // boundaries and are never mistaken for a failed chunk import.
    if (failed) return createElement(RouteLoadError)
    return Page ? createElement(Page, props) : createElement(RouteLoading)
  }
}
