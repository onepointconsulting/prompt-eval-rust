"use client";

import { Card } from "@/components/ui/Card";
import type { TrendPoint } from "@/lib/types";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

type PerformanceChartProps = {
  data: TrendPoint[];
};

export function PerformanceChart({ data }: PerformanceChartProps) {
  if (!data.length) {
    return (
      <Card>
        <h3 className="mb-4 text-sm font-semibold text-slate-900">Performance Over Time</h3>
        <p className="py-12 text-center text-sm text-slate-500">No trend data yet.</p>
      </Card>
    );
  }
  return (
    <Card>
      <h3 className="mb-4 text-sm font-semibold text-slate-900">Performance Over Time</h3>
      <div className="h-72">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e2e8f0" />
            <XAxis dataKey="date" tick={{ fill: "#64748b", fontSize: 12 }} />
            <YAxis domain={[0, 10]} tick={{ fill: "#64748b", fontSize: 12 }} />
            <Tooltip />
            <Line
              type="monotone"
              dataKey="score"
              stroke="#2563eb"
              strokeWidth={3}
              dot={{ fill: "#2563eb", r: 4 }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </Card>
  );
}
