import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import type { DatasetItem } from "@/lib/types";
import { EllipsisVerticalIcon } from "@heroicons/react/24/outline";

type DatasetCardProps = {
  dataset: DatasetItem;
  onView?: (id: string) => void;
  onEdit?: (id: string) => void;
  onQuickEval?: (id: string) => void;
  onDelete?: (id: string) => void;
};

export function DatasetCard({
  dataset,
  onView,
  onEdit,
  onQuickEval,
  onDelete,
}: DatasetCardProps) {
  return (
    <Card className="space-y-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-base font-semibold text-slate-900">{dataset.name}</h3>
          <p className="text-sm text-slate-500">
            {dataset.questions} questions • Last used {dataset.lastUsed}
          </p>
        </div>
        <Button variant="ghost" size="icon">
          <EllipsisVerticalIcon className="h-4 w-4" />
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <Badge variant="default">
          Avg score: {dataset.avgScore ? dataset.avgScore.toFixed(1) : "N/A"}
        </Badge>
        <Badge variant="neutral">{dataset.evaluations} evaluations</Badge>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button variant="secondary" size="sm" onClick={() => onView?.(dataset.id)}>
          View
        </Button>
        <Button variant="secondary" size="sm" onClick={() => onEdit?.(dataset.id)}>
          Edit
        </Button>
        <Button size="sm" onClick={() => onQuickEval?.(dataset.id)}>
          Quick Eval
        </Button>
        <Button variant="danger" size="sm" onClick={() => onDelete?.(dataset.id)}>
          Delete
        </Button>
      </div>
    </Card>
  );
}
