"use client";

import { cn } from "@/lib/utils";

type Tab = {
  id: string;
  label: string;
};

type TabsProps = {
  value: string;
  onChange: (value: string) => void;
  tabs: Tab[];
};

export function Tabs({ value, onChange, tabs }: TabsProps) {
  return (
    <div className="inline-flex rounded-lg border border-slate-200 bg-white p-1">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onChange(tab.id)}
          className={cn(
            "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
            value === tab.id
              ? "bg-blue-600 text-white"
              : "text-slate-600 hover:bg-slate-100"
          )}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
