import { Card } from "@/components/ui/Card";
import type { QuestionComparison } from "@/lib/types";
import { StrengthsWeaknessesList } from "./StrengthsWeaknessesList";
import ReactMarkdown from "react-markdown";

function MetaList({ meta }: { meta?: Record<string, unknown> }) {
  const entries = Object.entries(meta ?? {}).filter(([, v]) => v !== null && v !== undefined);
  if (!entries.length) return null;
  return (
    <details className="mt-3 rounded-lg border bg-slate-50 px-3 py-2">
      <summary className="cursor-pointer text-xs font-semibold text-slate-700">
        Details
      </summary>
      <dl className="mt-2 grid gap-x-4 gap-y-1 text-xs text-slate-600 sm:grid-cols-2">
        {entries.map(([k, v]) => (
          <div key={k} className="flex items-start justify-between gap-2">
            <dt className="font-medium text-slate-700">{k}</dt>
            <dd className="text-right font-mono text-[11px] text-slate-600">
              {String(v)}
            </dd>
          </div>
        ))}
      </dl>
    </details>
  );
}

type QuestionDetailProps = {
  item: QuestionComparison;
};

export function QuestionDetail({ item }: QuestionDetailProps) {
  return (
    <Card>
      <h3 className="mb-4 text-sm font-semibold text-slate-900">{item.question}</h3>
      <div className={`grid gap-4 ${item.promptB ? "lg:grid-cols-2" : ""}`}>
        <div className="rounded-lg border p-4">
          <p className="font-semibold text-slate-900">
            {item.promptA.name} ({item.promptA.score.toFixed(1)})
          </p>
          <div className="prose prose-slate mt-2 max-w-none text-sm">
            <ReactMarkdown>{item.promptA.response}</ReactMarkdown>
          </div>
          <MetaList meta={item.promptA.meta} />
          <div className="mt-3">
            <StrengthsWeaknessesList
              strengths={item.promptA.strengths}
              weaknesses={item.promptA.weaknesses}
            />
          </div>
        </div>
        {item.promptB ? (
          <div className="rounded-lg border p-4">
            <p className="font-semibold text-slate-900">
              {item.promptB.name} ({item.promptB.score.toFixed(1)})
            </p>
            <div className="prose prose-slate mt-2 max-w-none text-sm">
              <ReactMarkdown>{item.promptB.response}</ReactMarkdown>
            </div>
            <MetaList meta={item.promptB.meta} />
            <div className="mt-3">
              <StrengthsWeaknessesList
                strengths={item.promptB.strengths}
                weaknesses={item.promptB.weaknesses}
              />
            </div>
          </div>
        ) : null}
      </div>
    </Card>
  );
}
