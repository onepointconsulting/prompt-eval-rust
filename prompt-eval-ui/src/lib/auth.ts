import type { NextAuthOptions } from "next-auth";
import CredentialsProvider from "next-auth/providers/credentials";

/** The user shape NextAuth carries through `authorize` → jwt → session. */
type AuthorizedUser = {
  id: string;
  email: string;
  name: string | null;
  /** Backend-issued JWT, attached as a Bearer token on every API call. */
  accessToken: string;
};

/**
 * Verify an email/password pair against the Rust backend's POST /api/auth/login.
 * On success the backend returns a signed JWT plus the user record; that JWT
 * becomes the Bearer token the frontend sends on every subsequent API request.
 * Returns null on bad credentials (backend 401) or any backend/network error.
 */
async function verifyCredentials(
  email: string,
  password: string
): Promise<AuthorizedUser | null> {
  // Runs server-side (inside the ui container), so it needs an absolute,
  // container-reachable URL — not the browser's relative /api path.
  const base =
    process.env.INTERNAL_API_BASE_URL ??
    process.env.NEXT_PUBLIC_API_BASE_URL ??
    "http://127.0.0.1:3001/rust-api";
  try {
    const url = `${base}/auth/login`
    console.info("fetching from backend: ", url);
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });
    console.info("response from backend: ", res);
    if (!res.ok) return null;
    const data = (await res.json()) as {
      token: string;
      user: { id: string; email: string; name: string | null };
    };
    return {
      id: data.user.id,
      email: data.user.email,
      name: data.user.name,
      accessToken: data.token,
    };
  } catch (e) {
    console.error("Backend login request failed:", e);
    return null;
  }
}

export const authOptions: NextAuthOptions = {
  // JWT sessions — no DB needed on the frontend. The token rides in an HttpOnly
  // cookie that the proxy reads to gate routes.
  session: { strategy: "jwt" },
  pages: { signIn: "/login" },
  providers: [
    CredentialsProvider({
      name: "Email and password",
      credentials: {
        email: { label: "Email", type: "email" },
        password: { label: "Password", type: "password" },
      },
      async authorize(credentials) {
        if (!credentials?.email || !credentials?.password) return null;
        return verifyCredentials(credentials.email, credentials.password);
      },
    }),
  ],
  callbacks: {
    // At sign-in, carry the user id and the backend JWT onto the NextAuth token.
    async jwt({ token, user }) {
      if (user) {
        const u = user as AuthorizedUser;
        token.id = u.id;
        token.accessToken = u.accessToken;
      }
      return token;
    },
    // Expose them on the session: `session.user.id` for display and
    // `session.accessToken` for the Bearer header in api.ts.
    async session({ session, token }) {
      if (session.user) {
        (session.user as { id?: string }).id = token.id as string | undefined;
      }
      (session as { accessToken?: string }).accessToken =
        token.accessToken as string | undefined;
      return session;
    },
  },
};
