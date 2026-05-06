"use client";

import type { PromptTemplate } from "@/lib/types";

type PromptSelectorProps = {
  prompts: PromptTemplate[];
  selected: string[];
  onToggle: (id: string) => void;
};

export function PromptSelector({ prompts, selected, onToggle }: PromptSelectorProps) {
  return (
    <div className="space-y-2">
      {prompts.map((prompt) => (
        <label key={prompt.id} className="flex cursor-pointer items-center gap-3 rounded-lg border p-3">
          <input
            type="checkbox"
            checked={selected.includes(prompt.id)}
            onChange={() => onToggle(prompt.id)}
          />
          <div>
            <p className="text-sm font-medium text-slate-900">{prompt.name}</p>
            <p className="text-xs text-slate-500">
              Score: {prompt.avgScore?.toFixed(1) ?? "Not tested"}
            </p>
          </div>
        </label>
      ))}
    </div>
  );
}
