"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { PromptEditor } from "@/components/prompts/PromptEditor";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { api } from "@/lib/api";
import type { GeneratedTestCase } from "@/lib/types";
import { ArrowLeftIcon } from "@heroicons/react/24/outline";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";

const DUPLICATE_STORAGE_KEY = "promptEval:duplicatePrompt";

export default function PromptGeneratePage() {
  const router = useRouter();
  const [description, setDescription] = useState("");
  const [promptName, setPromptName] = useState("AG Mentor - Socratic Teaching v1");
  const [datasetName, setDatasetName] = useState("AG Mentor - Generated Test Cases");
  const [promptStatus, setPromptStatus] = useState<"active" | "draft" | "archived">("active");
  const [editorValue, setEditorValue] = useState("");
  const [generatedVariables, setGeneratedVariables] = useState<string[]>([]);
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
        };
        if (typeof d.content === "string" && d.content.trim()) {
          setEditorValue(d.content);
          if (typeof d.name === "string" && d.name.trim()) {
            setPromptName(d.name.trim());
          }
          if (Array.isArray(d.variables) && d.variables.every((x) => typeof x === "string")) {
            setGeneratedVariables(d.variables);
          }
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
      });
      setLastSavedPromptId(created.id);
      toast.success("Prompt created. You can generate test cases below.");
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
      const questions = generatedCases.map((c, idx) => {
        const qValue = c.variable_values?.QUESTION;
        const question =
          typeof qValue === "string" && qValue.trim().length
            ? qValue
            : `Generated test case ${idx + 1}`;
        return {
          question,
          answer: null,
          variable_values: c.variable_values,
          tags: c.tags,
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
      <div className="grid gap-3 rounded-xl border bg-white p-4">
        <p className="text-sm font-semibold text-slate-900">
          Step 1: Generate from description (optional)
        </p>
        <p className="text-xs text-slate-500">
          Or paste or type a template in the editor below without using AI.
        </p>
        <Input
          placeholder="Describe what this prompt should do..."
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
        <div className="flex justify-end">
          <Button onClick={onGeneratePrompt} disabled={isGenerating} type="button">
            {isGenerating ? "Generating..." : "Generate with AI"}
          </Button>
        </div>
      </div>

      <div className="space-y-2">
        <p className="text-sm font-semibold text-slate-900">Step 2: Edit template</p>
        <PromptEditor value={editorValue} onChange={setEditorValue} />
      </div>

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

      {generatedVariables.length > 0 ? (
        <div className="rounded-xl border bg-white p-4">
          <p className="text-sm font-semibold text-slate-900">Detected variables</p>
          <div className="mt-2 flex flex-wrap gap-2">
            {generatedVariables.map((v) => (
              <span
                key={v}
                className="rounded-full border border-slate-200 bg-slate-50 px-2.5 py-1 text-xs text-slate-700"
              >
                {v}
              </span>
            ))}
          </div>
        </div>
      ) : null}

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
        {!lastSavedPromptId ? (
          <p className="mt-2 text-sm text-slate-500">
            Save the prompt (step 3) first, then return here or stay on this page to generate cases.
          </p>
        ) : null}
        {generatedCases.length > 0 ? (
          <>
            <div className="mt-3 grid gap-3 sm:grid-cols-[1fr_auto]">
              <Input
                placeholder="Dataset name"
                value={datasetName}
                onChange={(e) => setDatasetName(e.target.value)}
              />
              <Button onClick={onSaveCasesAsDataset} disabled={isSavingDataset} type="button">
                {isSavingDataset ? "Saving..." : "Save test cases as dataset"}
              </Button>
            </div>
            <pre className="mt-3 overflow-x-auto rounded-lg bg-slate-900 p-3 text-xs text-slate-100">
              {JSON.stringify(generatedCases, null, 2)}
            </pre>
          </>
        ) : null}
      </div>
    </PageContainer>
  );
}
