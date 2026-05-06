export type PromptVariant = {
  id: string;
  name: string;
  template: string;
  status: "baseline" | "candidate";
};

export type EvalDatasetItem = {
  id: number;
  question: string;
};

export type EvalRunItem = {
  id: number;
  question: string;
  response: string;
  score: number;
};

export type ScoreSummary = {
  promptName: string;
  average: number;
  min: number;
  max: number;
};
