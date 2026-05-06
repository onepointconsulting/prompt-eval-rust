"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { PromptEditor } from "@/components/prompts/PromptEditor";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { api } from "@/lib/api";
import { useParams, useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

export default function PromptDetailPage() {
  const router = useRouter();
  const params = useParams();
  const id = typeof params.id === "string" ? params.id : "";

  const [name, setName] = useState("");
  const [status, setStatus] = useState<"active" | "draft" | "archived">("draft");
  const [template, setTemplate] = useState("");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    if (!id) {
      setLoadError("Missing prompt id.");
      setLoading(false);
      return;
    }
    setLoading(true);
    setLoadError(null);
    try {
      const p = await api.getPrompt(id);
      setName(p.name);
      setStatus(p.status);
      setTemplate(p.content);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : "Failed to load prompt.");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    const t = window.setTimeout(() => {
      void load();
    }, 0);
    return () => window.clearTimeout(t);
  }, [load]);

  const onSave = async () => {
    const trimmedName = name.trim();
    const trimmedTemplate = template.trim();
    if (!trimmedName) {
      toast.error("Name is required.");
      return;
    }
    if (!trimmedTemplate) {
      toast.error("Template is required.");
      return;
    }
    try {
      setSaving(true);
      await api.updatePrompt(id, {
        name: trimmedName,
        template: trimmedTemplate,
        status,
      });
      toast.success("Prompt saved.");
      router.push("/prompts");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to save prompt.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <PageContainer title="Prompt Editor" description="Edit and test an individual prompt.">
      {loading ? (
        <p className="text-sm text-slate-500">Loading prompt…</p>
      ) : loadError ? (
        <div className="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-800">
          <p>{loadError}</p>
          <div className="mt-3 flex gap-2">
            <Button type="button" size="sm" variant="secondary" onClick={() => router.push("/prompts")}>
              Back to library
            </Button>
            <Button type="button" size="sm" onClick={() => load()}>
              Retry
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="grid gap-3 rounded-xl border bg-white p-4 sm:grid-cols-[1fr_200px]">
            <Input
              placeholder="Prompt name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <Select
              value={status}
              onChange={(e) => setStatus(e.target.value as "active" | "draft" | "archived")}
            >
              <option value="active">Active</option>
              <option value="draft">Draft</option>
              <option value="archived">Archived</option>
            </Select>
          </div>
          <PromptEditor
            value={template}
            onChange={setTemplate}
            onSave={onSave}
            onCancel={() => router.push("/prompts")}
            isSaving={saving}
          />
        </div>
      )}
    </PageContainer>
  );
}
