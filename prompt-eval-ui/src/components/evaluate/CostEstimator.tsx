import { Card } from "@/components/ui/Card";

export function CostEstimator() {
  return (
    <Card>
      <h3 className="text-sm font-semibold text-slate-900">Estimated Run Cost</h3>
      <p className="mt-2 text-sm text-slate-600">Cost: $0.05</p>
      <p className="text-sm text-slate-600">Time: ~2 minutes</p>
      <p className="text-sm text-slate-600">API Calls: 15</p>
    </Card>
  );
}
