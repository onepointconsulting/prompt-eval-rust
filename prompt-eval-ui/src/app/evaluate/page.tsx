"use client";

import { AdvancedSettings } from "@/components/evaluate/AdvancedSettings";
import { CostEstimator } from "@/components/evaluate/CostEstimator";
import { EvaluationProgress } from "@/components/evaluate/EvaluationProgress";
import { PageContainer } from "@/components/layout/PageContainer";
import { PromptSelector } from "@/components/prompts/PromptSelector";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Select } from "@/components/ui/Select";
import { useApiData } from "@/hooks/useApiData";
import { api } from "@/lib/api";
import { formatLocalDateTime, formatRelativeTime } from "@/lib/time";
import type { DatasetItem, EvalSummary } from "@/lib/types";
import { toast } from "sonner";
import { useEffect, useMemo, useRef, useState } from "react";

export default function EvaluatePage() {
  const [selectedPrompts, setSelectedPrompts] = useState<string[]>([]);
  const [selectedDataset, setSelectedDataset] = useState("");
  const [runResult, setRunResult] = useState<EvalSummary | null>(null);
  const [evaluating, setEvaluating] = useState(false);
  const promptsData = useApiData(api.getPrompts, [], "Prompts");
  const datasetsData = useApiData(api.getDatasets, [] as DatasetItem[], "Datasets");
  const availablePrompts = useMemo(() => promptsData.data ?? [], [promptsData.data]);
  const availableDatasets = useMemo(() => datasetsData.data ?? [], [datasetsData.data]);
  const effectiveDatasetId = selectedDataset || availableDatasets[0]?.id || "";
  const effectiveDataset = useMemo(
    () => availableDatasets.find((d) => d.id === effectiveDatasetId) ?? null,
    [availableDatasets, effectiveDatasetId]
  );
  const totalWork =
    (effectiveDataset?.questions ?? 0) * (selectedPrompts.length > 0 ? selectedPrompts.length : 0);

  const promptsFromApi = promptsData.data;
  const seededPromptSelection = useRef(false);
  useEffect(() => {
    if (seededPromptSelection.current || !promptsFromApi?.length) return;
    seededPromptSelection.current = true;
    setSelectedPrompts(promptsFromApi.slice(0, 2).map((p) => p.id));
  }, [promptsFromApi]);

  const togglePrompt = (id: string) => {
    setSelectedPrompts((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
    );
  };

  const onStartEvaluation = async () => {
    if (!selectedPrompts.length) {
      toast.error("Select at least one prompt.");
      return;
    }
    if (!effectiveDatasetId) {
      toast.error("Select a dataset first.");
      return;
    }
    try {
      setEvaluating(true);
      const res = await api.runEvaluation({
        dataset_id: effectiveDatasetId,
        prompt_ids: selectedPrompts,
      });
      setRunResult({
        id: res.id,
        dataset: res.dataset,
        promptNames: res.prompts,
        winner: res.prompts[0] ?? "N/A",
        score: res.average_score,
        createdAt: res.created_at,
      });
      toast.success("Evaluation completed.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to run evaluation.");
    } finally {
      setEvaluating(false);
    }
  };

  return (
    <PageContainer
      title="Run Evaluation"
      description="Select a dataset and prompts, then run a full comparison."
    >
      <Card>
        <h3 className="mb-2 text-sm font-semibold text-slate-900">Step 1: Select Dataset</h3>
        <Select
          value={effectiveDatasetId}
          onChange={(e) => setSelectedDataset(e.target.value)}
        >
          {availableDatasets.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name} ({d.questions} questions)
            </option>
          ))}
        </Select>
      </Card>

      <Card>
        <h3 className="mb-2 text-sm font-semibold text-slate-900">
          Step 2: Select Prompts to Compare
        </h3>
        <PromptSelector
          prompts={availablePrompts}
          selected={selectedPrompts}
          onToggle={togglePrompt}
        />
      </Card>

      <AdvancedSettings />
      <CostEstimator />

      <div className="flex flex-wrap gap-2">
        <Button onClick={onStartEvaluation} disabled={evaluating}>
          {evaluating ? "Evaluating…" : "Start Evaluation"}
        </Button>
        <Button
          variant="secondary"
          onClick={() => toast.info("Configuration saved.")}
          disabled={evaluating}
        >
          Save Configuration
        </Button>
      </div>

      <EvaluationProgress running={evaluating} current={evaluating ? 0 : totalWork} total={totalWork} />
      {runResult && (
        <Card>
          <p className="text-sm font-semibold text-slate-900">Latest Run</p>
          <p className="text-sm text-slate-600">
            {runResult.id} • score {runResult.score.toFixed(2)} •{" "}
            {formatRelativeTime(runResult.createdAt)}
          </p>
          <p className="text-xs text-slate-500">{formatLocalDateTime(runResult.createdAt)}</p>
        </Card>
      )}
    </PageContainer>
  );
}
