import { createFileRoute } from '@tanstack/react-router'

import App from '@/App'
import { AuthGate } from '@/features/auth/AuthGate'

export const Route = createFileRoute('/')({ component: IndexRoute })

function IndexRoute() {
  return (
    <AuthGate>
      {({ user }) => <App authUser={user} />}
    </AuthGate>
  )
}
