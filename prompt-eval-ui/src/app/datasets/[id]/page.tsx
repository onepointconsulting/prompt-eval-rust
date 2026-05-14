"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { api } from "@/lib/api";
import type { DatasetItem, DatasetQuestion } from "@/lib/types";
import { useParams, useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

export default function DatasetDetailPage() {
  const router = useRouter();
  const params = useParams();
  const id = typeof params.id === "string" ? params.id : "";

  const [dataset, setDataset] = useState<DatasetItem | null>(null);
  const [questions, setQuestions] = useState<DatasetQuestion[]>([]);
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    if (!id) { setLoadError("Missing dataset id."); setLoading(false); return; }
    setLoading(true);
    setLoadError(null);
    try {
      const [ds, qs] = await Promise.all([api.getDataset(id), api.getDatasetQuestions(id)]);
      setDataset(ds);
      setName(ds.name);
      setQuestions(qs);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : "Failed to load dataset.");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    const t = window.setTimeout(() => { void load(); }, 0);
    return () => window.clearTimeout(t);
  }, [load]);

  const onSave = async () => {
    const trimmed = name.trim();
    if (!trimmed) { toast.error("Name is required."); return; }
    setSaving(true);
    try {
      const updated = await api.updateDataset(id, { name: trimmed });
      setDataset(updated);
      toast.success("Dataset renamed.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to update dataset.");
    } finally {
      setSaving(false);
    }
  };

  const onDelete = async () => {
    if (!confirm(`Delete "${dataset?.name}"? This will remove all questions and cannot be undone.`)) return;
    try {
      await api.deleteDataset(id);
      toast.success("Dataset deleted.");
      router.push("/datasets");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to delete dataset.");
    }
  };

  if (loading) {
    return (
      <PageContainer title="Dataset" description="">
        <p className="text-sm text-slate-500">Loading…</p>
      </PageContainer>
    );
  }

  if (loadError || !dataset) {
    return (
      <PageContainer title="Dataset" description="">
        <div className="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-800">
          <p>{loadError ?? "Dataset not found."}</p>
          <div className="mt-3 flex gap-2">
            <Button type="button" size="sm" variant="secondary" onClick={() => router.push("/datasets")}>
              Back to datasets
            </Button>
            <Button type="button" size="sm" onClick={() => void load()}>
              Retry
            </Button>
          </div>
        </div>
      </PageContainer>
    );
  }

  return (
    <PageContainer
      title={dataset.name}
      description={`${dataset.questions} questions · ${dataset.evaluations} evaluation${dataset.evaluations !== 1 ? "s" : ""}${dataset.createdAt ? ` · created ${dataset.createdAt.slice(0, 10)}` : ""}`}
    >
      {/* Rename */}
      <div className="grid gap-3 rounded-xl border bg-white p-4 sm:grid-cols-[1fr_auto_auto]">
        <Input
          placeholder="Dataset name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <Button onClick={() => void onSave()} disabled={saving || name.trim() === dataset.name}>
          {saving ? "Saving…" : "Rename"}
        </Button>
        <Button variant="danger" onClick={() => void onDelete()}>
          Delete
        </Button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-4">
        {[
          { label: "Questions", value: dataset.questions },
          { label: "Evaluations", value: dataset.evaluations },
          { label: "Avg Score", value: dataset.avgScore != null ? dataset.avgScore.toFixed(1) : "—" },
        ].map((s) => (
          <div key={s.label} className="rounded-xl border bg-white p-4 text-center">
            <p className="text-2xl font-bold text-slate-800">{s.value}</p>
            <p className="mt-0.5 text-sm text-slate-500">{s.label}</p>
          </div>
        ))}
      </div>

      {/* Questions table */}
      <div className="overflow-hidden rounded-xl border bg-white">
        <div className="border-b px-4 py-3">
          <p className="text-sm font-semibold text-slate-800">Questions</p>
        </div>
        {questions.length === 0 ? (
          <p className="px-4 py-8 text-center text-sm text-slate-500">No questions in this dataset.</p>
        ) : (
          <table className="w-full text-sm">
            <thead className="border-b bg-slate-50 text-left text-xs font-medium uppercase tracking-wide text-slate-500">
              <tr>
                <th className="px-4 py-3 w-10">#</th>
                <th className="px-4 py-3">Question</th>
                <th className="px-4 py-3 w-24">Difficulty</th>
                <th className="px-4 py-3 w-36">Type</th>
                <th className="px-4 py-3">Tags</th>
              </tr>
            </thead>
            <tbody className="divide-y">
              {questions.map((q, i) => (
                <tr key={q.id} className="hover:bg-slate-50">
                  <td className="px-4 py-3 text-slate-400">{i + 1}</td>
                  <td className="px-4 py-3 max-w-md text-slate-800">{q.questionText}</td>
                  <td className="px-4 py-3">
                    {q.difficulty ? (
                      <span className={`rounded px-2 py-0.5 text-xs font-medium ${difficultyClass(q.difficulty)}`}>
                        {q.difficulty}
                      </span>
                    ) : (
                      <span className="text-slate-400">—</span>
                    )}
                  </td>
                  <td className="px-4 py-3 text-slate-600">{q.caseType ?? <span className="text-slate-400">—</span>}</td>
                  <td className="px-4 py-3 text-slate-500 text-xs">{q.tags?.join(", ") ?? <span className="text-slate-400">—</span>}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </PageContainer>
  );
}

function difficultyClass(d: string) {
  switch (d) {
    case "easy":         return "bg-green-100 text-green-700";
    case "medium":       return "bg-yellow-100 text-yellow-700";
    case "hard":         return "bg-orange-100 text-orange-700";
    case "adversarial":  return "bg-red-100 text-red-700";
    default:             return "bg-slate-100 text-slate-600";
  }
}
