"use client";

import { Card } from "@/components/ui/Card";
import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";

type ComparisonChartProps = {
  data: { name: string; score: number }[];
};

export function ComparisonChart({ data }: ComparisonChartProps) {
  if (!data.length) {
    return (
      <Card>
        <h3 className="mb-4 text-sm font-semibold text-slate-900">Overall Comparison</h3>
        <p className="py-12 text-center text-sm text-slate-500">No comparison data.</p>
      </Card>
    );
  }
  return (
    <Card>
      <h3 className="mb-4 text-sm font-semibold text-slate-900">Overall Comparison</h3>
      <div className="h-72">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="name" />
            <YAxis domain={[0, 10]} />
            <Tooltip />
            <Bar dataKey="score" fill="#2563eb" radius={[8, 8, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </Card>
  );
}
