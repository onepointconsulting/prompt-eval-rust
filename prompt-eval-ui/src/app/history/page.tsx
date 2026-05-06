"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { useApiData } from "@/hooks/useApiData";
import { api } from "@/lib/api";
import { formatLocalDateTime } from "@/lib/time";
import type { EvalSummary } from "@/lib/types";

const emptyHistory: EvalSummary[] = [];

export default function HistoryPage() {
  const history = useApiData(api.getHistory, emptyHistory, "Evaluation history");

  return (
    <PageContainer
      title="Evaluation History"
      description="Track all historical evaluation runs."
      actions={<Button variant="secondary">Export</Button>}
    >
      <div className="grid gap-3 sm:grid-cols-3">
        <Input placeholder="Search history..." />
        <Select defaultValue="all">
          <option value="all">All datasets</option>
        </Select>
        <Button variant="secondary">Apply Filters</Button>
      </div>
      <Card className="overflow-x-auto p-0">
        <table className="w-full min-w-[820px] text-left text-sm">
          <thead className="border-b bg-slate-50 text-slate-600">
            <tr>
              <th className="px-4 py-3">ID</th>
              <th className="px-4 py-3">Date</th>
              <th className="px-4 py-3">Dataset</th>
              <th className="px-4 py-3">Prompts</th>
              <th className="px-4 py-3">Winner</th>
              <th className="px-4 py-3">Score</th>
            </tr>
          </thead>
          <tbody>
            {(history.data ?? emptyHistory).length === 0 ? (
              <tr>
                <td className="px-4 py-8 text-slate-500" colSpan={6}>
                  No evaluation history yet.
                </td>
              </tr>
            ) : (
              (history.data ?? emptyHistory).map((row) => (
                <tr key={row.id} className="border-b">
                  <td className="px-4 py-3">#{row.id}</td>
                  <td className="px-4 py-3">{formatLocalDateTime(row.createdAt)}</td>
                  <td className="px-4 py-3">{row.dataset}</td>
                  <td className="px-4 py-3">{row.promptNames.join(" vs ")}</td>
                  <td className="px-4 py-3">{row.winner}</td>
                  <td className="px-4 py-3">{row.score.toFixed(1)}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </Card>
    </PageContainer>
  );
}
