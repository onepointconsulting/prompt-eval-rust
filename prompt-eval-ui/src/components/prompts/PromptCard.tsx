import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import type { PromptTemplate } from "@/lib/types";

type PromptCardProps = {
  prompt: PromptTemplate;
  onEdit?: (id: string) => void;
  onDuplicate?: (id: string) => void;
  onTest?: (id: string) => void;
  onDelete?: (id: string) => void;
};

export function PromptCard({
  prompt,
  onEdit,
  onDuplicate,
  onTest,
  onDelete,
}: PromptCardProps) {
  return (
    <Card className="space-y-3">
      <div className="flex items-start justify-between gap-3">
        <h3 className="text-base font-semibold text-slate-900">{prompt.name}</h3>
        <Badge
          variant={
            prompt.status === "active"
              ? "success"
              : prompt.status === "draft"
                ? "warning"
                : "neutral"
          }
        >
          {prompt.status}
        </Badge>
      </div>
      <pre className="overflow-x-auto rounded-md bg-slate-900 p-3 text-xs text-slate-100">
        {prompt.content}
      </pre>
      <p className="text-sm text-slate-500">
        Avg score: {prompt.avgScore?.toFixed(1) ?? "Not tested"} • Used {prompt.runs} times
      </p>
      <div className="flex flex-wrap gap-2">
        <Button variant="secondary" size="sm" onClick={() => onEdit?.(prompt.id)}>
          Edit
        </Button>
        <Button variant="secondary" size="sm" onClick={() => onDuplicate?.(prompt.id)}>
          Duplicate
        </Button>
        <Button size="sm" onClick={() => onTest?.(prompt.id)}>
          Test
        </Button>
        <Button variant="danger" size="sm" onClick={() => onDelete?.(prompt.id)}>
          Delete
        </Button>
      </div>
    </Card>
  );
}
