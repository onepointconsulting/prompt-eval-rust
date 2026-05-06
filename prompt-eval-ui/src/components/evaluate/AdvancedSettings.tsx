"use client";

import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { useState } from "react";

export function AdvancedSettings() {
  const [open, setOpen] = useState(true);

  if (!open) {
    return (
      <button className="text-sm text-blue-700" onClick={() => setOpen(true)}>
        Show Advanced Settings
      </button>
    );
  }

  return (
    <div className="rounded-xl border bg-white p-4">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-slate-900">Advanced Settings</h3>
        <button className="text-xs text-slate-500" onClick={() => setOpen(false)}>
          Hide
        </button>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <p className="mb-1 text-xs text-slate-500">Model</p>
          <Select defaultValue="claude-sonnet-4-20250514">
            <option>claude-sonnet-4-20250514</option>
            <option>claude-haiku-4-5-20251001</option>
          </Select>
        </div>
        <div>
          <p className="mb-1 text-xs text-slate-500">Grading model</p>
          <Select defaultValue="claude-haiku-4-5-20251001">
            <option>claude-haiku-4-5-20251001</option>
            <option>claude-sonnet-4-20250514</option>
          </Select>
        </div>
        <div>
          <p className="mb-1 text-xs text-slate-500">Max tokens</p>
          <Input defaultValue={2000} type="number" />
        </div>
        <div>
          <p className="mb-1 text-xs text-slate-500">Temperature</p>
          <Input defaultValue={0.7} type="number" step="0.1" />
        </div>
      </div>
    </div>
  );
}
