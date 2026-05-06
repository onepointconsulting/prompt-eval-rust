"use client";

import { DatasetCard } from "@/components/datasets/DatasetCard";
import { DatasetPreview } from "@/components/datasets/DatasetPreview";
import { DatasetUploader } from "@/components/datasets/DatasetUploader";
import { PageContainer } from "@/components/layout/PageContainer";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { useApiData } from "@/hooks/useApiData";
import { api } from "@/lib/api";
import type { DatasetItem } from "@/lib/types";
import { PlusIcon } from "@heroicons/react/24/outline";
import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";

export default function DatasetsPage() {
  const router = useRouter();
  const datasets = useApiData(api.getDatasets, [] as DatasetItem[], "Datasets");
  const [createdItems, setCreatedItems] = useState<DatasetItem[]>([]);
  const [deletedIds, setDeletedIds] = useState<Set<string>>(new Set());
  const [name, setName] = useState("");
  const [questionCount, setQuestionCount] = useState(5);
  const items = useMemo(() => {
    const combined = [...createdItems, ...(datasets.data ?? [])];
    return combined.filter((d) => !deletedIds.has(d.id));
  }, [createdItems, datasets.data, deletedIds]);

  const onCreateDataset = async () => {
    if (!name.trim()) {
      toast.error("Dataset name is required.");
      return;
    }
    try {
      const created = await api.createDataset({
        name: name.trim(),
        question_count: questionCount,
      });
      setCreatedItems((prev) => [created, ...prev]);
      setName("");
      toast.success("Dataset created.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to create dataset.");
    }
  };

  const onDeleteDataset = async (id: string) => {
    try {
      await api.deleteDataset(id);
      setCreatedItems((prev) => prev.filter((d) => d.id !== id));
      setDeletedIds((prev) => new Set(prev).add(id));
      toast.success("Dataset deleted.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to delete dataset.");
    }
  };

  return (
    <PageContainer
      title="Datasets"
      description="Manage evaluation datasets and upload new question sets."
      actions={
        <Button>
          <PlusIcon className="mr-2 h-4 w-4" />
          Upload New
        </Button>
      }
    >
      <div className="max-w-md">
        <Input placeholder="Search datasets..." />
      </div>
      <div className="grid gap-3 rounded-xl border bg-white p-4 sm:grid-cols-[1fr_180px_auto]">
        <Input
          placeholder="new_dataset.json"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <Input
          type="number"
          min={1}
          value={questionCount}
          onChange={(e) => setQuestionCount(Number(e.target.value))}
        />
        <Button onClick={onCreateDataset}>Create Dataset</Button>
      </div>
      <div className="space-y-4">
        {items.map((dataset) => (
          <DatasetCard
            key={dataset.id}
            dataset={dataset}
            onView={(id) => router.push(`/datasets/${id}`)}
            onEdit={(id) => router.push(`/datasets/${id}`)}
            onQuickEval={() => router.push("/evaluate")}
            onDelete={onDeleteDataset}
          />
        ))}
      </div>
      <DatasetUploader />
      <DatasetPreview sample="[]" />
    </PageContainer>
  );
}
