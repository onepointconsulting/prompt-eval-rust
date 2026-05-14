"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { PromptEditor } from "@/components/prompts/PromptEditor";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { api } from "@/lib/api";
import type { GeneratedTestCase, RubricCriterion } from "@/lib/types";
import { ArrowLeftIcon } from "@heroicons/react/24/outline";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";

const DUPLICATE_STORAGE_KEY = "promptEval:duplicatePrompt";

export default function PromptGeneratePage() {
  const router = useRouter();
  const [description, setDescription] = useState("");
  const [promptName, setPromptName] = useState("Generated Prompt v1");
  const [datasetName, setDatasetName] = useState("Generated Test Cases");
  const [promptStatus, setPromptStatus] = useState<"active" | "draft" | "archived">("active");
  const [editorValue, setEditorValue] = useState("");
  const [generatedVariables, setGeneratedVariables] = useState<string[]>([]);
  const [generatedDomain, setGeneratedDomain] = useState<string | null>(null);
  const [generatedRubric, setGeneratedRubric] = useState<RubricCriterion[]>([]);
  const [generatedOutputFormat, setGeneratedOutputFormat] = useState<string | null>(null);
  const [generatedCases, setGeneratedCases] = useState<GeneratedTestCase[]>([]);
  const [lastSavedPromptId, setLastSavedPromptId] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isGeneratingCases, setIsGeneratingCases] = useState(false);
  const [isSavingDataset, setIsSavingDataset] = useState(false);

  useEffect(() => {
    const t = window.setTimeout(() => {
      const raw = sessionStorage.getItem(DUPLICATE_STORAGE_KEY);
      if (!raw) return;
      sessionStorage.removeItem(DUPLICATE_STORAGE_KEY);
      try {
        const d = JSON.parse(raw) as {
          name?: string;
          content?: string;
          variables?: string[];
          domain?: string;
          rubric?: RubricCriterion[];
          expectedOutputFormat?: string;
        };
        if (typeof d.content === "string" && d.content.trim()) {
          setEditorValue(d.content);
          if (typeof d.name === "string" && d.name.trim()) setPromptName(d.name.trim());
          if (Array.isArray(d.variables) && d.variables.every((x) => typeof x === "string")) {
            setGeneratedVariables(d.variables);
          }
          if (typeof d.domain === "string") setGeneratedDomain(d.domain);
          if (Array.isArray(d.rubric)) setGeneratedRubric(d.rubric);
          if (typeof d.expectedOutputFormat === "string") setGeneratedOutputFormat(d.expectedOutputFormat);
          toast.success("Template loaded from duplicate. Adjust and save.");
        }
      } catch {
        // ignore malformed payload
      }
    }, 0);
    return () => window.clearTimeout(t);
  }, []);

  const onGeneratePrompt = async () => {
    const text = description.trim();
    if (!text) {
      toast.error("Enter a description first.");
      return;
    }
    try {
      setIsGenerating(true);
      const generated = await api.generatePrompt(text);
      setEditorValue(generated.template);
      setGeneratedVariables(generated.variables);
      setGeneratedDomain(generated.domain);
      setGeneratedRubric(generated.rubric);
      setGeneratedOutputFormat(generated.expectedOutputFormat);
      toast.success("Prompt generated. Review and edit before saving.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to generate prompt.");
    } finally {
      setIsGenerating(false);
    }
  };

  const onCreatePrompt = async () => {
    const content = editorValue.trim();
    if (!content) {
      toast.error("Prompt content is required.");
      return;
    }
    const name = promptName.trim();
    if (!name) {
      toast.error("Prompt name is required.");
      return;
    }
    try {
      setIsSaving(true);
      const created = await api.createPrompt({
        name,
        template: content,
        variables: generatedVariables,
        is_templated: generatedVariables.length > 0,
        status: promptStatus,
        domain: generatedDomain ?? undefined,
        rubric: generatedRubric.length > 0 ? generatedRubric : undefined,
        expected_output_format: generatedOutputFormat ?? undefined,
      });
      setLastSavedPromptId(created.id);
      toast.success("Prompt saved. You can now generate test cases below.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to create prompt.");
    } finally {
      setIsSaving(false);
    }
  };

  const onGenerateCases = async () => {
    if (!lastSavedPromptId) {
      toast.error("Save the prompt first.");
      return;
    }
    try {
      setIsGeneratingCases(true);
      const testCases = await api.generateTestCases({
        prompt_id: lastSavedPromptId,
        count: 5,
      });
      setGeneratedCases(testCases);
      toast.success(`Generated ${testCases.length} test cases.`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to generate test cases.");
    } finally {
      setIsGeneratingCases(false);
    }
  };

  const onSaveCasesAsDataset = async () => {
    if (generatedCases.length === 0) {
      toast.error("Generate test cases first.");
      return;
    }
    const name = datasetName.trim();
    if (!name) {
      toast.error("Dataset name is required.");
      return;
    }
    try {
      setIsSavingDataset(true);
      // Pick the best question text from variable_values.
      // Try well-known keys first, then the longest string value in the object.
      const PREFERRED_KEYS = ["USER_MESSAGE", "QUESTION", "STUDENT_MESSAGE", "QUERY", "INPUT", "MESSAGE"];
      const questions = generatedCases.map((c, idx) => {
        const vars = c.variable_values ?? {};
        const fromPreferred = PREFERRED_KEYS
          .map((k) => vars[k])
          .find((v): v is string => typeof v === "string" && v.trim().length > 0);
        const fromAny = fromPreferred ??
          Object.values(vars)
            .filter((v): v is string => typeof v === "string" && v.trim().length > 0)
            .sort((a, b) => b.length - a.length)[0];
        const questionText = fromAny?.trim() || `Generated test case ${idx + 1}`;
        return {
          question: questionText,
          answer: c.expected_answer ?? null,
          variable_values: c.variable_values,
          tags: c.tags,
          difficulty: c.difficulty,
          case_type: c.case_type,
          reasoning: c.reasoning,
        };
      });
      const created = await api.createDatasetFromQuestions({
        name,
        description: `Generated from prompt ${lastSavedPromptId ?? "unknown"}`,
        questions,
      });
      toast.success(`Saved dataset "${created.name}" with ${created.questions} questions.`);
      router.push("/datasets");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to save dataset.");
    } finally {
      setIsSavingDataset(false);
    }
  };

  return (
    <PageContainer
      title="Generate prompt"
      description="Describe goals, refine the template, save, then optionally generate test cases and a dataset."
      actions={
        <Button variant="secondary" type="button" onClick={() => router.push("/prompts")}>
          <ArrowLeftIcon className="mr-2 h-4 w-4" />
          Back to library
        </Button>
      }
    >
      {/* Step 1 */}
      <div className="grid gap-3 rounded-xl border bg-white p-4">
        <p className="text-sm font-semibold text-slate-900">
          Step 1: Generate from description (optional)
        </p>
        <p className="text-xs text-slate-500">
          Or paste / type a template in the editor below without using AI.
        </p>
        <textarea
          placeholder="Describe what this prompt should do..."
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          className="min-h-60 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm outline-none transition-all focus:border-blue-500 focus:ring-2 focus:ring-blue-100"
        />
        <div className="flex justify-end">
          <Button onClick={onGeneratePrompt} disabled={isGenerating} type="button">
            {isGenerating ? "Generating..." : "Generate with AI"}
          </Button>
        </div>
      </div>

      {/* Step 2 — editor */}
      {editorValue.length > 0 && (
        <div className="space-y-2">
          <p className="text-sm font-semibold text-slate-900">Step 2: Edit template</p>
          <PromptEditor value={editorValue} onChange={setEditorValue} />
        </div>
      )}

      {/* Step 3 — name & save */}
      <div className="grid gap-3 rounded-xl border bg-white p-4 sm:grid-cols-3">
        <p className="text-sm font-semibold text-slate-900 sm:col-span-3">Step 3: Name and save</p>
        <Input
          placeholder="Prompt name"
          value={promptName}
          onChange={(e) => setPromptName(e.target.value)}
        />
        <Select
          value={promptStatus}
          onChange={(e) => setPromptStatus(e.target.value as "active" | "draft" | "archived")}
        >
          <option value="active">Active</option>
          <option value="draft">Draft</option>
          <option value="archived">Archived</option>
        </Select>
        <div className="flex justify-end">
          <Button onClick={onCreatePrompt} disabled={isSaving} type="button">
            {isSaving ? "Saving..." : "Save prompt"}
          </Button>
        </div>
      </div>

      {/* Variables */}
      {generatedVariables.length > 0 && (
        <div className="rounded-xl border bg-white p-4">
          <p className="text-sm font-semibold text-slate-900">Detected variables</p>
          <div className="mt-2 flex flex-wrap gap-2">
            {generatedVariables.map((v) => (
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

      {/* Domain + Rubric (shown after AI generation) */}
      {(generatedDomain || generatedRubric.length > 0 || generatedOutputFormat) && (
        <div className="rounded-xl border bg-white p-4 space-y-3">
          <p className="text-sm font-semibold text-slate-900">Generated metadata</p>

          {generatedDomain && (
            <div className="flex items-center gap-2">
              <span className="text-xs text-slate-500">Domain:</span>
              <span className="rounded-full bg-violet-100 px-2.5 py-0.5 text-xs font-medium text-violet-700">
                {generatedDomain}
              </span>
            </div>
          )}

          {generatedRubric.length > 0 && (
            <div>
              <p className="text-xs font-semibold text-slate-600 mb-2">Evaluation rubric</p>
              <div className="space-y-2">
                {generatedRubric.map((r) => (
                  <div key={r.name} className="rounded-lg border border-slate-100 bg-slate-50 px-3 py-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-semibold text-slate-800">{r.name}</span>
                      <span className="text-xs text-slate-500">weight {(r.weight * 100).toFixed(0)}%</span>
                    </div>
                    <p className="mt-0.5 text-xs text-slate-600">{r.description}</p>
                  </div>
                ))}
              </div>
            </div>
          )}

          {generatedOutputFormat && (
            <div>
              <p className="text-xs font-semibold text-slate-600 mb-1">Expected output format</p>
              <p className="text-xs text-slate-600 rounded-lg bg-slate-50 border px-3 py-2">{generatedOutputFormat}</p>
            </div>
          )}
        </div>
      )}

      {/* Step 4 — test cases */}
      <div className="rounded-xl border bg-white p-4">
        <div className="flex items-center justify-between">
          <p className="text-sm font-semibold text-slate-900">Step 4: Generate test cases</p>
          <Button
            variant="secondary"
            onClick={onGenerateCases}
            disabled={!lastSavedPromptId || isGeneratingCases}
            type="button"
          >
            {isGeneratingCases ? "Generating..." : "Generate test cases"}
          </Button>
        </div>
        {!lastSavedPromptId && (
          <p className="mt-2 text-sm text-slate-500">
            Save the prompt first, then generate test cases.
          </p>
        )}
        {generatedCases.length > 0 && (
          <>
            <div className="mt-3 grid gap-3 sm:grid-cols-[1fr_auto]">
              <Input
                placeholder="Dataset name"
                value={datasetName}
                onChange={(e) => setDatasetName(e.target.value)}
              />
              <Button onClick={onSaveCasesAsDataset} disabled={isSavingDataset} type="button">
                {isSavingDataset ? "Saving..." : "Save as dataset"}
              </Button>
            </div>
            {/* Preview generated cases */}
            <div className="mt-3 space-y-2">
              {generatedCases.map((c, i) => (
                <div key={i} className="rounded-lg border border-slate-100 bg-slate-50 px-3 py-2 text-xs">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="font-semibold text-slate-700">Case {i + 1}</span>
                    <span className="rounded bg-amber-100 px-1.5 py-0.5 text-amber-700">{c.difficulty}</span>
                    <span className="rounded bg-slate-200 px-1.5 py-0.5 text-slate-600">{c.case_type}</span>
                  </div>
                  {c.expected_answer && (
                    <p className="text-slate-600">
                      <span className="font-medium">Expected: </span>{c.expected_answer}
                    </p>
                  )}
                  <p className="mt-0.5 text-slate-500 italic">{c.reasoning}</p>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </PageContainer>
  );
}
