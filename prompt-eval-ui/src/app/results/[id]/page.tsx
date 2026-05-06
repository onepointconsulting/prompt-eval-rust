"use client";

import { ComparisonChart } from "@/components/results/ComparisonChart";
import { ExportOptions } from "@/components/results/ExportOptions";
import { QuestionDetail } from "@/components/results/QuestionDetail";
import { Card } from "@/components/ui/Card";
import { Tabs } from "@/components/ui/Tabs";
import { useApiData } from "@/hooks/useApiData";
import { api } from "@/lib/api";
import type { ResultsDetail } from "@/lib/types";
import Link from "next/link";
import { useParams } from "next/navigation";
import { useCallback, useState } from "react";

const emptyDetail: ResultsDetail = {
  id: "",
  dataset: "",
  promptScores: [],
  questions: [],
};

export default function ResultDetailPage() {
  const params = useParams();
  const id = typeof params?.id === "string" ? params.id : "";
  const load = useCallback(() => {
    if (!id) return Promise.resolve(emptyDetail);
    return api.getResultDetail(id);
  }, [id]);
  const detail = useApiData(load, emptyDetail, "Result detail");

  const [tab, setTab] = useState("overview");
  const data = detail.data ?? emptyDetail;
  const hasData = Boolean(data.id);

  return (
    <div className="space-y-5">
      <div>
        <Link href="/results" className="text-sm text-blue-700 hover:underline">
          ← Back to Results
        </Link>
        {detail.loading && (
          <Card className="mt-2 text-sm text-slate-500">Loading evaluation…</Card>
        )}
        {detail.error && (
          <Card className="mt-2 border-orange-200 bg-orange-50 text-sm text-orange-800">
            {detail.error}
          </Card>
        )}
        <h1 className="mt-1 text-2xl font-bold text-slate-900">
          {hasData ? `Evaluation #${data.id}` : "Evaluation"}
        </h1>
        <p className="text-sm text-slate-600">{hasData ? data.dataset : "—"}</p>
      </div>

      {hasData && (
        <>
          <ComparisonChart data={data.promptScores} />

          <Tabs
            value={tab}
            onChange={setTab}
            tabs={[
              { id: "overview", label: "Overview" },
              { id: "question", label: "Question by Question" },
              { id: "raw", label: "Raw Data" },
              { id: "export", label: "Export" },
            ]}
          />

          {(tab === "overview" || tab === "question") &&
            (data.questions.length > 0 ? (
              data.questions.map((q) => <QuestionDetail key={q.question} item={q} />)
            ) : (
              <Card className="text-sm text-slate-500">
                No per-question breakdown for this run yet.
              </Card>
            ))}
          {tab === "raw" && (
            <pre className="overflow-x-auto rounded-xl border bg-slate-900 p-4 text-xs text-slate-100">
              {JSON.stringify(data, null, 2)}
            </pre>
          )}
          {tab === "export" && <ExportOptions />}
        </>
      )}
    </div>
  );
}
