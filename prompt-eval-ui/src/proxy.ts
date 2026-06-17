import { getToken } from "next-auth/jwt";
import { NextResponse, type NextRequest } from "next/server";

// Routes reachable without a session. Everything else requires auth.
const PUBLIC_PATHS = ["/login", "/docs"];

/**
 * Route protection. In Next.js 16 this file is "proxy" (formerly middleware) and
 * runs on the Node.js runtime, so next-auth's getToken — which verifies the JWT
 * session cookie — works here directly.
 *
 * This is an optimistic, cookie-only gate (the docs' recommended use of proxy);

 */
export default async function proxy(req: NextRequest) {
  const { pathname } = req.nextUrl;
  const isPublic = PUBLIC_PATHS.some(
    (p) => pathname === p || pathname.startsWith(`${p}/`)
  );

  const token = await getToken({ req, secret: process.env.NEXTAUTH_SECRET });
  console.log("token in proxy: ", token);
  const isAuthed = !!token;

  // Not signed in, heading for a protected route → /login.
  if (!isAuthed && !isPublic) {
    const url = new URL("/login", req.url);
    url.searchParams.set("callbackUrl", pathname);
    return NextResponse.redirect(url);
  }

  // Already signed in but sitting on /login → send to the dashboard.
  if (isAuthed && isPublic) {
    return NextResponse.redirect(new URL("/", req.url));
  }

  return NextResponse.next();
}

export const config = {
  // Skip Next internals, static assets, and the /api/auth/* routes (which must
  // stay reachable for sign-in/out and session checks).
  matcher: ["/((?!api|_next/static|_next/image|favicon.ico).*)"],
};
