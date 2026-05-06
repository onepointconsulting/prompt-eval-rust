import { Card } from "@/components/ui/Card";
import { Progress } from "@/components/ui/Progress";

type EvaluationProgressProps = {
  running?: boolean;
  current: number;
  total: number;
};

export function EvaluationProgress({ running = false, current, total }: EvaluationProgressProps) {
  const percent = total === 0 ? 0 : (current / total) * 100;

  return (
    <Card>
      <p className="mb-2 text-sm font-semibold text-slate-900">
        {running ? "Evaluating…" : "Progress"}{" "}
        <span className="font-normal text-slate-600">
          {total > 0 ? `${current}/${total}` : "—"}
        </span>
      </p>
      {running ? (
        <div className="h-2 w-full rounded-full bg-slate-200 overflow-hidden">
          <div className="h-2 w-1/3 rounded-full bg-blue-600 animate-pulse" />
        </div>
      ) : (
        <Progress value={percent} />
      )}
      <p className="mt-3 text-sm text-slate-600">
        {running
          ? "Running the selected prompts on the dataset and grading responses."
          : total === 0
            ? "Select a dataset and at least one prompt to start."
            : "Ready."}
      </p>
    </Card>
  );
}
