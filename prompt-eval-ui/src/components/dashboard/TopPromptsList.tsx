import { Badge } from "@/components/ui/Badge";
import { Card } from "@/components/ui/Card";
import type { PromptTemplate } from "@/lib/types";

type TopPromptsListProps = {
  prompts: PromptTemplate[];
};

export function TopPromptsList({ prompts }: TopPromptsListProps) {
  return (
    <Card>
      <h3 className="mb-4 text-sm font-semibold text-slate-900">Top Performing Prompts</h3>
      {prompts.length === 0 ? (
        <p className="text-sm text-slate-500">No prompts loaded yet.</p>
      ) : null}
      <div className="space-y-3">
        {prompts.slice(0, 4).map((prompt, index) => (
          <div key={prompt.id} className="flex items-center justify-between rounded-lg border p-3">
            <div>
              <p className="text-sm font-semibold text-slate-900">
                {index + 1}. {prompt.name}
              </p>
              <p className="text-xs text-slate-500">{prompt.runs} runs</p>
            </div>
            <Badge variant="success">{(prompt.avgScore ?? 0).toFixed(1)}</Badge>
          </div>
        ))}
      </div>
    </Card>
  );
}
