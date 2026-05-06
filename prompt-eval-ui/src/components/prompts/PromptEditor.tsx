"use client";

import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";

type PromptEditorProps = {
  value: string;
  onChange: (value: string) => void;
  /** When set, renders Save/Cancel row wired to these handlers */
  onSave?: () => void | Promise<void>;
  onCancel?: () => void;
  isSaving?: boolean;
};

export function PromptEditor({
  value,
  onChange,
  onSave,
  onCancel,
  isSaving,
}: PromptEditorProps) {
  const showFooter = Boolean(onSave || onCancel);

  return (
    <Card>
      <h3 className="mb-3 text-sm font-semibold text-slate-900">Prompt Editor</h3>
      <textarea
        className="min-h-40 w-full rounded-lg border border-slate-300 p-3 text-sm outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-100"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      {showFooter ? (
        <div className="mt-3 flex gap-2">
          {onSave ? (
            <Button size="sm" type="button" disabled={isSaving} onClick={() => void onSave()}>
              {isSaving ? "Saving…" : "Save"}
            </Button>
          ) : null}
          {onCancel ? (
            <Button size="sm" variant="secondary" type="button" disabled={isSaving} onClick={onCancel}>
              Cancel
            </Button>
          ) : null}
        </div>
      ) : null}
    </Card>
  );
}
