import { Card } from "@/components/ui/Card";
import type { QuestionComparison } from "@/lib/types";
import { StrengthsWeaknessesList } from "./StrengthsWeaknessesList";
import ReactMarkdown from "react-markdown";

type PromptResult = NonNullable<QuestionComparison["promptA"]>;

function DimensionScores({ scores }: { scores: PromptResult["dimensionScores"] }) {
  if (!scores || Object.keys(scores).length === 0) return null;
  return (
    <div className="mt-3">
      <p className="mb-1.5 text-xs font-semibold text-slate-600">Per-criterion scores</p>
      <div className="space-y-1.5">
        {Object.entries(scores).map(([name, dim]) => (
          <div key={name} className="rounded-lg border border-slate-100 bg-slate-50 px-3 py-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold text-slate-700">{name}</span>
              <span className="text-xs font-bold text-blue-700">{dim.score.toFixed(1)}</span>
            </div>
            <p className="mt-0.5 text-xs text-slate-500">{dim.reasoning}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function JudgePanel({ result }: { result: PromptResult }) {
  return (
    <div className="rounded-lg border p-4">
      <div className="flex items-center justify-between gap-2 mb-3">
        <p className="font-semibold text-slate-900">
          {result.name} — {result.score.toFixed(1)} / 10
        </p>
        {result.referenceUsed !== undefined && (
          <span
            className={`rounded-full px-2 py-0.5 text-xs font-medium ${
              result.referenceUsed
                ? "bg-green-100 text-green-700"
                : "bg-slate-100 text-slate-600"
            }`}
          >
            {result.referenceUsed ? "reference used" : "no reference"}
          </span>
        )}
      </div>

      <div className="prose prose-slate max-w-none text-sm">
        <ReactMarkdown>{result.response}</ReactMarkdown>
      </div>

      {result.judgeReasoning && (
        <div className="mt-3 rounded-lg border border-amber-100 bg-amber-50 px-3 py-2 text-xs text-amber-800">
          <span className="font-semibold">Judge reasoning: </span>
          {result.judgeReasoning}
        </div>
      )}

      <DimensionScores scores={result.dimensionScores} />

      <div className="mt-3">
        <StrengthsWeaknessesList
          strengths={result.strengths}
          weaknesses={result.weaknesses}
        />
      </div>
    </div>
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
        <JudgePanel result={item.promptA} />
        {item.promptB && <JudgePanel result={item.promptB} />}
      </div>
    </Card>
  );
}
