export type RubricCriterion = {
  name: string;
  description: string;
  weight: number;
};

export type DimensionScore = {
  score: number;
  reasoning: string;
};

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
  domain?: string;
  rubric?: RubricCriterion[];
  expectedOutputFormat?: string;
};

export type GeneratedPrompt = {
  template: string;
  variables: string[];
  domain: string;
  rubric: RubricCriterion[];
  expectedOutputFormat: string;
};

export type GeneratedTestCase = {
  variable_values: Record<string, unknown>;
  expected_answer?: string;
  difficulty: string;
  case_type: string;
  tags: string[];
  reasoning: string;
};

export type DatasetItem = {
  id: string;
  name: string;
  questions: number;
  avgScore?: number;
  evaluations: number;
  lastUsed: string;
  createdAt?: string;
};

export type DatasetQuestion = {
  id: number;
  datasetId: string;
  questionText: string;
  expectedAnswer?: string;
  questionOrder: number;
  variableValues?: Record<string, unknown>;
  tags?: string[];
  difficulty?: string;
  caseType?: string;
};

export type QuestionComparison = {
  question: string;
  promptA: {
    name: string;
    score: number;
    response: string;
    strengths: string[];
    weaknesses: string[];
    dimensionScores?: Record<string, DimensionScore>;
    judgeReasoning?: string;
    referenceUsed?: boolean;
    meta?: Record<string, string | number | boolean | null | undefined>;
  };
  promptB?: {
    name: string;
    score: number;
    response: string;
    strengths: string[];
    weaknesses: string[];
    dimensionScores?: Record<string, DimensionScore>;
    judgeReasoning?: string;
    referenceUsed?: boolean;
    meta?: Record<string, string | number | boolean | null | undefined>;
  };
};

export type ResultsDetail = {
  id: string;
  dataset: string;
  promptScores: { name: string; score: number }[];
  questions: QuestionComparison[];
};
