"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { PromptCard } from "@/components/prompts/PromptCard";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { useApiData } from "@/hooks/useApiData";
import { api } from "@/lib/api";
import type { PromptTemplate } from "@/lib/types";
import { PlusIcon, SparklesIcon } from "@heroicons/react/24/outline";
import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";

const DUPLICATE_PROMPT_STORAGE_KEY = "promptEval:duplicatePrompt";

export default function PromptsPage() {
  const router = useRouter();
  const [createdItems, setCreatedItems] = useState<PromptTemplate[]>([]);
  const [deletedIds, setDeletedIds] = useState<Set<string>>(new Set());
  const prompts = useApiData(api.getPrompts, [] as PromptTemplate[], "Prompts");
  const items = useMemo(
    () =>
      [...createdItems, ...(prompts.data ?? [])].filter((p) => !deletedIds.has(p.id)),
    [createdItems, prompts.data, deletedIds]
  );

  const onDeletePrompt = async (id: string) => {
    try {
      await api.deletePrompt(id);
      setCreatedItems((prev) => prev.filter((p) => p.id !== id));
      setDeletedIds((prev) => new Set(prev).add(id));
      toast.success("Prompt deleted.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to delete prompt.");
    }
  };

  return (
    <PageContainer
      title="Prompt Templates"
      description="Create, test, and compare prompt templates."
      actions={
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            type="button"
            onClick={() => router.push("/prompts/generate")}
          >
            <SparklesIcon className="mr-2 h-4 w-4" />
            Generate with AI
          </Button>
          <Button type="button" onClick={() => router.push("/prompts/generate")}>
            <PlusIcon className="mr-2 h-4 w-4" />
            New prompt
          </Button>
        </div>
      }
    >
      <div className="grid gap-3 sm:grid-cols-3">
        <Input placeholder="Search prompt templates..." />
        <Select defaultValue="all">
          <option value="all">All</option>
          <option value="active">Active</option>
          <option value="draft">Draft</option>
          <option value="archived">Archived</option>
        </Select>
        <Select defaultValue="performance">
          <option value="performance">Sort by performance</option>
          <option value="recent">Sort by recent</option>
        </Select>
      </div>
      <div className="space-y-4">
        {items.map((prompt) => (
          <PromptCard
            key={prompt.id}
            prompt={prompt}
            onEdit={(id) => router.push(`/prompts/${id}`)}
            onDuplicate={(id) => {
              const source = items.find((p) => p.id === id);
              if (!source) return;
              try {
                sessionStorage.setItem(
                  DUPLICATE_PROMPT_STORAGE_KEY,
                  JSON.stringify({
                    name: `${source.name} (copy)`,
                    content: source.content,
                    variables: source.variables ?? [],
                    domain: source.domain,
                    rubric: source.rubric,
                    expectedOutputFormat: source.expectedOutputFormat,
                  })
                );
                router.push("/prompts/generate");
                toast.success("Open the generate page to edit the duplicate.");
              } catch {
                toast.error("Could not duplicate; try copying the template manually.");
              }
            }}
            onTest={() => router.push("/evaluate")}
            onDelete={onDeletePrompt}
          />
        ))}
      </div>
    </PageContainer>
  );
}
