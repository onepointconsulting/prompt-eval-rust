import { Card } from "@/components/ui/Card";
import type { ComponentType } from "react";

type StatCardProps = {
  title: string;
  value: string;
  hint?: string;
  icon: ComponentType<{ className?: string }>;
};

export function StatCard({ title, value, hint, icon: Icon }: StatCardProps) {
  return (
    <Card className="p-5">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm text-slate-500">{title}</p>
          <p className="mt-2 text-2xl font-bold text-slate-900">{value}</p>
          {hint && <p className="mt-1 text-xs text-slate-500">{hint}</p>}
        </div>
        <div className="rounded-lg bg-blue-50 p-2 text-blue-700">
          <Icon className="h-5 w-5" />
        </div>
      </div>
    </Card>
  );
}
