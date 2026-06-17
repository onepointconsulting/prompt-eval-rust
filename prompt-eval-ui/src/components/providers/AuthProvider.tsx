"use client";

import { SessionProvider } from "next-auth/react";
import type { ReactNode } from "react";

/**
 * Client boundary for next-auth's SessionProvider. The root layout is a Server
 * Component, so this React-context provider must live behind a "use client"
 * wrapper. SessionProvider fetches the session from /api/auth/session itself —
 * we don't pass a session prop down from the server.
 */

export function AuthProvider({ children }: { children: ReactNode }) {
  return <SessionProvider>{children}</SessionProvider>;
}
