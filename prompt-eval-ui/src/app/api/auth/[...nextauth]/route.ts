import NextAuth from "next-auth";
import { authOptions } from "@/lib/auth";

// next-auth v4 exposes one handler that serves both GET and POST for every
// /api/auth/* route (signin, callback, session, signout, csrf, ...).
const handler = NextAuth(authOptions);

export { handler as GET, handler as POST };
