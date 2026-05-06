import { Card } from "@/components/ui/Card";

type DatasetPreviewProps = {
  sample: string;
};

export function DatasetPreview({ sample }: DatasetPreviewProps) {
  return (
    <Card>
      <h3 className="mb-3 text-sm font-semibold text-slate-900">Dataset Format Preview</h3>
      <pre className="overflow-x-auto rounded-lg bg-slate-900 p-4 text-xs text-slate-100">
        {sample}
      </pre>
    </Card>
  );
}
