import { Card } from "@/components/ui/Card";
import { formatLocalDateTime } from "@/lib/time";
import type { EvalSummary } from "@/lib/types";

type RecentEvalsListProps = {
  items: EvalSummary[];
};

export function RecentEvalsList({ items }: RecentEvalsListProps) {
  return (
    <Card>
      <h3 className="mb-4 text-sm font-semibold text-slate-900">Recent Evaluations</h3>
      {items.length === 0 ? (
        <p className="text-sm text-slate-500">No evaluations yet.</p>
      ) : null}
      <div className="space-y-3">
        {items.slice(0, 4).map((item) => (
          <div key={item.id} className="rounded-lg border p-3">
            <p className="text-sm font-semibold text-slate-900">Eval #{item.id}</p>
            <p className="text-xs text-slate-500">
              {item.dataset} • {formatLocalDateTime(item.createdAt)}
            </p>
            <p className="mt-1 text-sm text-blue-700">Score: {item.score.toFixed(1)} / 10</p>
          </div>
        ))}
      </div>
    </Card>
  );
}
