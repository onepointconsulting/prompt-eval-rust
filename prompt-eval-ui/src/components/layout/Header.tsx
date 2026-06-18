"use client";

import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import {
  Bars3Icon,
  ChartBarIcon,
  ClockIcon,
  CpuChipIcon,
  DocumentTextIcon,
  HomeIcon,
  PlayIcon,
  Squares2X2Icon,
} from "@heroicons/react/24/outline";
import { signOut, useSession } from "next-auth/react";
import Link from "next/link";
import { usePathname } from "next/navigation";

const tabs = [
  { name: "Dashboard", href: "/", icon: HomeIcon },
  { name: "Datasets", href: "/datasets", icon: Squares2X2Icon },
  { name: "Prompts", href: "/prompts", icon: DocumentTextIcon },
  { name: "Evaluate", href: "/evaluate", icon: PlayIcon },
  { name: "Results", href: "/results", icon: ChartBarIcon },
  { name: "History", href: "/history", icon: ClockIcon },
];

type HeaderProps = {
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
};

export function Header({ onToggleSidebar }: HeaderProps) {
  const pathname = usePathname();
  const { data: session } = useSession();
  const email = session?.user?.email ?? "";
  const displayName = email
    .split("@")[0]
    .replace(/[._-]+/g, " ")
    .replace(/\b\w/g, c => c.toUpperCase());

  const initials = email ? email.slice(0, 2).toUpperCase() : "?";

  return (
    <header className="fixed inset-x-0 top-0 z-40 h-16 border-b bg-white/95 backdrop-blur">
      <div className="mx-auto flex h-full w-full max-w-[1600px] items-center justify-between px-4 sm:px-6 lg:px-8">
        <div className="flex items-center gap-3">
          <Button variant="ghost" size="icon" onClick={onToggleSidebar}>
            <Bars3Icon className="h-5 w-5" />
          </Button>
          <Link href="/" className="flex items-center gap-2">
            <div className="rounded-md bg-blue-600 p-1.5 text-white">
              <CpuChipIcon className="h-5 w-5" />
            </div>
            <span className="text-lg font-bold text-slate-900">PromptEval</span>
          </Link>
        </div>

        <nav className="hidden items-center gap-1 lg:flex">
          {tabs.map((tab) => {
            const active = pathname === tab.href;
            const Icon = tab.icon;
            return (
              <Link
                key={tab.href}
                href={tab.href}
                className={cn(
                  "flex items-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                  active
                    ? "bg-blue-50 text-blue-700"
                    : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
                )}
              >
                <Icon className="h-4 w-4" />
                {tab.name}
              </Link>
            );
          })}
        </nav>

        {email && (
          <div className="flex items-center gap-3">
            <div className="hidden text-right sm:block">
              <p className="text-sm font-semibold text-slate-900">{displayName}</p>
            </div>
            <span className="flex h-9 w-9 items-center justify-center rounded-full bg-slate-900 text-sm font-semibold text-white">
              {initials}
            </span>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => signOut({ callbackUrl: "/login" })}
            >
              Sign out
            </Button>
          </div>
        )}
      </div>
    </header>
  );
}
