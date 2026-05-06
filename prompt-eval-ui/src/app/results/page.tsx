"use client";

import { PageContainer } from "@/components/layout/PageContainer";
import { Badge } from "@/components/ui/Badge";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import { useApiData } from "@/hooks/useApiData";
import { api } from "@/lib/api";
import { formatLocalDateTime } from "@/lib/time";
import type { EvalSummary } from "@/lib/types";
import Link from "next/link";

const emptyResults: EvalSummary[] = [];

export default function ResultsPage() {
  const results = useApiData(api.getResultsList, emptyResults, "Results");
  return (
    <PageContainer
      title="Results"
      description="Browse and compare completed evaluation runs."
    >
      <div className="max-w-md">
        <Input placeholder="Search results..." />
      </div>
      <Card className="overflow-x-auto p-0">
        <table className="w-full min-w-[720px] text-left text-sm">
          <thead className="border-b bg-slate-50 text-slate-600">
            <tr>
              <th className="px-4 py-3">ID</th>
              <th className="px-4 py-3">Date</th>
              <th className="px-4 py-3">Dataset</th>
              <th className="px-4 py-3">Prompts</th>
              <th className="px-4 py-3">Winner</th>
              <th className="px-4 py-3">Score</th>
              <th className="px-4 py-3">Action</th>
            </tr>
          </thead>
          <tbody>
            {(results.data ?? emptyResults).length === 0 ? (
              <tr>
                <td className="px-4 py-8 text-slate-500" colSpan={7}>
                  No results yet.
                </td>
              </tr>
            ) : (
              (results.data ?? emptyResults).map((row) => (
                <tr key={row.id} className="border-b">
                  <td className="px-4 py-3">#{row.id}</td>
                  <td className="px-4 py-3 text-xs text-slate-500">
                    {formatLocalDateTime(row.createdAt)}
                  </td>
                  <td className="px-4 py-3">{row.dataset}</td>
                  <td className="px-4 py-3">{row.promptNames.join(" vs ")}</td>
                  <td className="px-4 py-3">
                    <Badge variant="success">{row.winner}</Badge>
                  </td>
                  <td className="px-4 py-3">{row.score.toFixed(1)} / 10</td>
                  <td className="px-4 py-3">
                    <Link className="text-blue-700 hover:underline" href={`/results/${row.id}`}>
                      View Details
                    </Link>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </Card>
    </PageContainer>
  );
}
