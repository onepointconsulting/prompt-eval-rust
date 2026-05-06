"use client";

import { cn } from "@/lib/utils";
import {
  BoltIcon,
  ChartBarIcon,
  ClockIcon,
  Cog6ToothIcon,
  DocumentTextIcon,
  FolderIcon,
  HomeIcon,
  PlusIcon,
  SparklesIcon,
} from "@heroicons/react/24/outline";
import Link from "next/link";
import { usePathname } from "next/navigation";

type SidebarProps = {
  open: boolean;
};

const sections = [
  {
    title: "General",
    items: [
      { href: "/", label: "Home", icon: HomeIcon },
      { href: "/history", label: "History", icon: ClockIcon },
    ],
  },
  {
    title: "Datasets",
    items: [
      { href: "/datasets", label: "All Datasets", icon: FolderIcon },
      { href: "/datasets/new", label: "New Dataset", icon: PlusIcon },
    ],
  },
  {
    title: "Prompts",
    items: [
      { href: "/prompts", label: "Library", icon: DocumentTextIcon },
      { href: "/prompts/generate", label: "Generate with AI", icon: SparklesIcon },
      { href: "/evaluate", label: "Quick Eval", icon: BoltIcon },
    ],
  },
  {
    title: "Results",
    items: [
      { href: "/results", label: "Reports", icon: ChartBarIcon },
      { href: "/settings", label: "Settings", icon: Cog6ToothIcon },
    ],
  },
];

export function Sidebar({ open }: SidebarProps) {
  const pathname = usePathname();

  return (
    <aside
      className={cn(
        "sticky top-16 hidden h-[calc(100vh-4rem)] shrink-0 border-r bg-white transition-all md:block",
        open ? "w-60" : "w-16"
      )}
    >
      <div className="space-y-5 p-3">
        {sections.map((section) => (
          <div key={section.title}>
            {open && (
              <p className="mb-2 px-2 text-[11px] font-semibold uppercase tracking-wide text-slate-400">
                {section.title}
              </p>
            )}
            <div className="space-y-1">
              {section.items.map((item) => {
                const active =
                  pathname === item.href ||
                  (item.href === "/prompts" &&
                    pathname.startsWith("/prompts/") &&
                    !pathname.startsWith("/prompts/generate"));
                const Icon = item.icon;
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={cn(
                      "flex items-center rounded-md px-2.5 py-2 text-sm transition-colors",
                      active
                        ? "bg-blue-50 font-medium text-blue-700"
                        : "text-slate-600 hover:bg-slate-100 hover:text-slate-900",
                      !open && "justify-center"
                    )}
                  >
                    <Icon className="h-4 w-4 shrink-0" />
                    {open && <span className="ml-2 truncate">{item.label}</span>}
                  </Link>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </aside>
  );
}
