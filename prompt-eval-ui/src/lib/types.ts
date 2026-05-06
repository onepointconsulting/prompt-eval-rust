export type DashboardStats = {
  totalEvals: number;
  activePrompts: number;
  avgScore: number;
  successRate: number;
};

export type TrendPoint = {
  date: string;
  score: number;
};

export type EvalSummary = {
  id: string;
  dataset: string;
  promptNames: string[];
  winner: string;
  score: number;
  createdAt: string;
};

export type PromptTemplate = {
  id: string;
  name: string;
  content: string;
  status: "active" | "draft" | "archived";
  avgScore?: number;
  runs: number;
  updatedAt: string;
  variables?: string[];
  isTemplated?: boolean;
};

export type GeneratedPrompt = {
  template: string;
  variables: string[];
};

export type GeneratedTestCase = {
  variable_values: Record<string, unknown>;
  tags: string[];
};

export type DatasetItem = {
  id: string;
  name: string;
  questions: number;
  avgScore?: number;
  evaluations: number;
  lastUsed: string;
};

export type QuestionComparison = {
  question: string;
  promptA: {
    name: string;
    score: number;
    response: string;
    strengths: string[];
    weaknesses: string[];
    meta?: Record<string, string | number | boolean | null | undefined>;
  };
  promptB?: {
    name: string;
    score: number;
    response: string;
    strengths: string[];
    weaknesses: string[];
    meta?: Record<string, string | number | boolean | null | undefined>;
  };
};

export type ResultsDetail = {
  id: string;
  dataset: string;
  promptScores: { name: string; score: number }[];
  questions: QuestionComparison[];
};
