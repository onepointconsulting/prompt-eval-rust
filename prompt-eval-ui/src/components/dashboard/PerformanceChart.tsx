"use client";

import { useMemo } from "react";
import { Card } from "@/components/ui/Card";
import type { TrendPoint } from "@/lib/types";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

type PerformanceChartProps = {
  data: TrendPoint[];
};

type ChartPoint = {
  id: string;
  date: string;
  score: number;
  showDateLabel: boolean;
  dateLabel: string;
};

function formatScore(score: number): number {
  return Math.round(score * 10) / 10;
}

function formatDateLabel(date: string): string {
  const parsed = new Date(`${date}T12:00:00`);
  if (Number.isNaN(parsed.getTime())) return date;
  return parsed.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function prepareChartData(data: TrendPoint[]): ChartPoint[] {
  const sorted = [...data].sort((a, b) => a.date.localeCompare(b.date));

  return sorted.map((point, index) => ({
    id: `run-${index}`,
    date: point.date,
    score: formatScore(point.score),
    dateLabel: formatDateLabel(point.date),
    showDateLabel: index === 0 || sorted[index - 1].date !== point.date,
  }));
}

type TooltipProps = {
  active?: boolean;
  payload?: Array<{ payload: ChartPoint }>;
};

function ChartTooltip({ active, payload }: TooltipProps) {
  if (!active || !payload?.length) return null;

  const point = payload[0].payload;
  return (
    <div className="rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm shadow-md">
      <p className="font-medium text-slate-900">{point.dateLabel}</p>
      <p className="text-slate-600">
        Score: <span className="font-semibold text-blue-600">{point.score.toFixed(1)}</span>
      </p>
    </div>
  );
}

export function PerformanceChart({ data }: PerformanceChartProps) {
  const chartData = useMemo(() => prepareChartData(data), [data]);

  if (!chartData.length) {
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
          <AreaChart data={chartData} margin={{ top: 8, right: 12, left: 0, bottom: 0 }}>
            <defs>
              <linearGradient id="scoreGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#2563eb" stopOpacity={0.35} />
                <stop offset="100%" stopColor="#2563eb" stopOpacity={0.02} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#e2e8f0" vertical={false} />
            <XAxis
              dataKey="id"
              tick={{ fill: "#64748b", fontSize: 12 }}
              tickLine={false}
              axisLine={{ stroke: "#e2e8f0" }}
              interval={0}
              tickFormatter={(id) => {
                const point = chartData.find((entry) => entry.id === id);
                return point?.showDateLabel ? point.dateLabel : "";
              }}
            />
            <YAxis
              domain={[0, 10]}
              tick={{ fill: "#64748b", fontSize: 12 }}
              tickLine={false}
              axisLine={false}
              width={32}
            />
            <Tooltip content={<ChartTooltip />} />
            <Area
              type="monotone"
              dataKey="score"
              stroke="#2563eb"
              strokeWidth={2}
              fill="url(#scoreGradient)"
              dot={{ fill: "#2563eb", r: 4, strokeWidth: 0 }}
              activeDot={{ r: 6, fill: "#2563eb", stroke: "#fff", strokeWidth: 2 }}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </Card>
  );
}
