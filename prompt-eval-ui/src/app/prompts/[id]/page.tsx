"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { PromptEditor } from "@/components/prompts/PromptEditor";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { api } from "@/lib/api";
import type { RubricCriterion } from "@/lib/types";
import { InformationCircleIcon } from "@heroicons/react/24/outline";
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
  const [domain, setDomain] = useState("");
  const [rubric, setRubric] = useState<RubricCriterion[]>([]);
  const [expectedOutputFormat, setExpectedOutputFormat] = useState("");
  const [variables, setVariables] = useState<string[]>([]);
  const [useContext, setUseContext] = useState(false);
  const [contextProject, setContextProject] = useState("");
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
      setDomain(p.domain ?? "");
      setRubric(p.rubric ?? []);
      setExpectedOutputFormat(p.expectedOutputFormat ?? "");
      setVariables(p.variables ?? []);
      setUseContext(p.useContext ?? false);
      setContextProject(p.contextProject ?? "");
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : "Failed to load prompt.");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    const t = window.setTimeout(() => { void load(); }, 0);
    return () => window.clearTimeout(t);
  }, [load]);

  const onSave = async () => {
    const trimmedName = name.trim();
    const trimmedTemplate = template.trim();
    if (!trimmedName) { toast.error("Name is required."); return; }
    if (!trimmedTemplate) { toast.error("Template is required."); return; }
    try {
      setSaving(true);
      const cleanedRubric = rubric
        .map((r) => ({
          name: r.name.trim(),
          description: r.description.trim(),
          weight: r.weight,
        }))
        .filter((r) => r.name.length > 0);
      await api.updatePrompt(id, {
        name: trimmedName,
        template: trimmedTemplate,
        status,
        domain: domain.trim() || undefined,
        rubric: cleanedRubric,
        expected_output_format: expectedOutputFormat.trim() || undefined,
        use_context: useContext,
        context_project: contextProject.trim() || undefined,
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
          {/* Name + Status */}
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

          {/* Template editor */}
          <PromptEditor
            value={template}
            onChange={setTemplate}
            onSave={onSave}
            onCancel={() => router.push("/prompts")}
            isSaving={saving}
          />

          {/* Variables (read-only display) */}
          {variables.length > 0 && (
            <div className="rounded-xl border bg-white p-4">
              <p className="mb-2 text-sm font-semibold text-slate-900">Template variables</p>
              <div className="flex flex-wrap gap-2">
                {variables.map((v) => (
                  <span
                    key={v}
                    className="rounded-full border border-blue-200 bg-blue-50 px-2.5 py-1 text-xs text-blue-700"
                  >
                    {`{{${v}}}`}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Domain + Expected output */}
          <div className="grid gap-3 rounded-xl border bg-white p-4 sm:grid-cols-2">
            <div>
              <label className="mb-1 block text-xs font-semibold text-slate-600">
                Domain (snake_case, optional)
              </label>
              <Input
                placeholder="e.g. educational_assistant"
                value={domain}
                onChange={(e) => setDomain(e.target.value)}
              />
            </div>
            <div>
              <label className="mb-1 block text-xs font-semibold text-slate-600">
                Expected output format (optional)
              </label>
              <Input
                placeholder="e.g. One guiding question, 1-2 sentences"
                value={expectedOutputFormat}
                onChange={(e) => setExpectedOutputFormat(e.target.value)}
              />
            </div>
          </div>

          {/* Knowledge base context */}
          <div className="rounded-xl border bg-red-300/50 p-4">
            <div className="flex items-center justify-between">
              <div>
                {/* icon  */}
                <div className="flex items-center gap-2">
                  <InformationCircleIcon className="w-4 h-4 text-slate-900" />
                  <p className="text-sm font-semibold text-slate-900">Knowledge base context</p>
                </div>
                <p className="mt-0.5 text-xs text-slate-900">
                  Fetch relevant context from the LightRAG engine before each evaluation response.
                </p>
              </div>
              <label className="relative inline-flex cursor-pointer items-center">
                <input
                  type="checkbox"
                  className="peer sr-only"
                  checked={useContext}
                  onChange={(e) => setUseContext(e.target.checked)}
                />
                <div className="peer h-6 w-11 rounded-full bg-slate-200 after:absolute after:left-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full" />
              </label>
            </div>
            {useContext && (
              <div className="mt-3">
                <label className="mb-1 block text-xs font-semibold text-slate-900">
                  Project name
                </label>
                <Input
                  placeholder="e.g. our_project_osca"
                  value={contextProject}
                  onChange={(e) => setContextProject(e.target.value)}
                  className="max-w-sm"
                />
                <p className="mt-1 text-xs text-slate-900">
                  The LightRAG project to query. Must match a configured project on the context engine.
                </p>
              </div>
            )}
          </div>

          {/* Rubric (editable) */}
          <div className="rounded-xl border bg-white p-4">
            <div className="mb-3 flex items-start justify-between gap-3">
              <div>
                <p className="text-sm font-semibold text-slate-900">Evaluation rubric</p>
                <p className="mt-0.5 text-xs text-slate-500">
                  Criteria the judge scores against. Weights are relative and should sum to 100%.
                </p>
              </div>
              {rubric.length > 0 &&
                (() => {
                  const totalPct = Math.round(
                    rubric.reduce((s, r) => s + r.weight, 0) * 100
                  );
                  const ok = totalPct === 100;
                  return (
                    <span
                      className={`whitespace-nowrap text-xs font-medium ${ok ? "text-slate-500" : "text-amber-600"
                        }`}
                    >
                      Total: {totalPct}%{ok ? "" : " — should be 100%"}
                    </span>
                  );
                })()}
            </div>

            {rubric.length === 0 ? (
              <p className="text-xs text-slate-400">
                No criteria yet — the judge falls back to a generic
                relevance/accuracy/completeness/clarity rubric. Add criteria to score
                against this prompt&apos;s specific goals.
              </p>
            ) : (
              <div className="space-y-3">
                {rubric.map((r, i) => (
                  <div key={i} className="rounded-lg border border-slate-200 p-3">
                    <div className="grid gap-2 sm:grid-cols-[1fr_96px_auto] sm:items-center">
                      <Input
                        placeholder="Criterion name (e.g. relevance)"
                        value={r.name}
                        onChange={(e) =>
                          setRubric((prev) =>
                            prev.map((c, j) =>
                              j === i ? { ...c, name: e.target.value } : c
                            )
                          )
                        }
                      />
                      <div className="flex items-center gap-1">
                        <Input
                          type="number"
                          min={0}
                          max={100}
                          value={Math.round(r.weight * 100)}
                          onChange={(e) => {
                            const pct = Math.max(
                              0,
                              Math.min(100, parseInt(e.target.value) || 0)
                            );
                            setRubric((prev) =>
                              prev.map((c, j) =>
                                j === i ? { ...c, weight: pct / 100 } : c
                              )
                            );
                          }}
                        />
                        <span className="text-xs text-slate-500">%</span>
                      </div>
                      <Button
                        type="button"
                        size="sm"
                        variant="danger"
                        onClick={() =>
                          setRubric((prev) => prev.filter((_, j) => j !== i))
                        }
                      >
                        Remove
                      </Button>
                    </div>
                    <textarea
                      className="mt-2 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm outline-none transition-all focus:border-blue-500 focus:ring-2 focus:ring-blue-100"
                      rows={2}
                      placeholder="What does this criterion check? Include any conditions the judge must enforce."
                      value={r.description}
                      onChange={(e) =>
                        setRubric((prev) =>
                          prev.map((c, j) =>
                            j === i ? { ...c, description: e.target.value } : c
                          )
                        )
                      }
                    />
                  </div>
                ))}
              </div>
            )}

            <Button
              type="button"
              size="sm"
              variant="secondary"
              className="mt-3"
              onClick={() =>
                setRubric((prev) => [
                  ...prev,
                  { name: "", description: "", weight: 0 },
                ])
              }
            >
              + Add criterion
            </Button>
          </div>
        </div>
      )}
    </PageContainer>
  );
}
