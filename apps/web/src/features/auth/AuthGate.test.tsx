import { render, screen } from "@testing-library/react"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRouter,
} from "@tanstack/react-router"
import { describe, expect, it, vi } from "vitest"

import { AuthGate } from "@/features/auth/AuthGate"

const fetchMeMock = vi.fn()

vi.mock("@/features/auth/api", async () => {
  const actual = await vi.importActual<typeof import("@/features/auth/api")>(
    "@/features/auth/api"
  )
  return {
    ...actual,
    fetchMe: (...args: Parameters<typeof actual.fetchMe>) => fetchMeMock(...args),
  }
})

function renderAuthGate() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  })

  const rootRoute = createRootRoute({
    component: () => <AuthGate>{() => <div>ok</div>}</AuthGate>,
  })

  const router = createRouter({
    routeTree: rootRoute,
    history: createMemoryHistory({
      initialEntries: ["/"],
    }),
  })

  return render(
      <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
}

describe("AuthGate", () => {
  it("shows bootstrap guidance when no local admin exists", async () => {
    fetchMeMock.mockResolvedValueOnce({
      authenticated: false,
      expires_at: null,
      login_enabled: false,
      user: null,
    })

    renderAuthGate()

    expect(
      await screen.findByRole("heading", {
        name: "还没有可登录的管理员账号",
      })
    ).toBeTruthy()
    expect(
      screen.getByText(/DEEPPRINT_INITIAL_ADMIN_PASSWORD/)
    ).toBeTruthy()
  })
})
