import { Suspense, lazy } from 'react'
import { Outlet, createRootRoute } from '@tanstack/react-router'

import '../styles.css'

const TanStackRouterDevtoolsPanel = import.meta.env.DEV
  ? lazy(() =>
      import('@tanstack/react-router-devtools').then((module) => ({
        default: module.TanStackRouterDevtoolsPanel,
      })),
    )
  : null

const TanStackDevtools = import.meta.env.DEV
  ? lazy(() =>
      import('@tanstack/react-devtools').then((module) => ({
        default: module.TanStackDevtools,
      })),
    )
  : null

export const Route = createRootRoute({
  component: RootComponent,
})

function RootComponent() {
  return (
    <>
      <Outlet />
      {import.meta.env.DEV && TanStackDevtools && TanStackRouterDevtoolsPanel ? (
        <Suspense fallback={null}>
          <TanStackDevtools
            config={{
              position: 'bottom-right',
            }}
            plugins={[
              {
                name: 'TanStack Router',
                render: <TanStackRouterDevtoolsPanel />,
              },
            ]}
          />
        </Suspense>
      ) : null}
    </>
  )
}
