import type {
  DashboardStats,
  DatasetItem,
  DatasetQuestion,
  DimensionScore,
  EvalSummary,
  GeneratedPrompt,
  GeneratedTestCase,
  PromptTemplate,
  ResultsDetail,
  RubricCriterion,
  TrendPoint,
} from "@/lib/types";

const API_BASE = "http://127.0.0.1:3001/api";

async function request<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`API ${res.status}: ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

async function requestWithBody<T>(
  path: string,
  method: "POST" | "PUT" | "DELETE",
  body?: unknown
): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    method,
    cache: "no-store",
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    throw new Error(`API ${res.status}: ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

// ── Internal API shapes (snake_case from Rust backend) ─────────────────────────

type ApiStats = {
  total_evaluations: number;
  active_prompts: number;
  average_score: number;
  success_rate: number;
};

type ApiRubricCriterion = {
  name: string;
  description: string;
  weight: number;
};

type ApiPrompt = {
  id: string;
  name: string;
  template: string;
  variables?: string[] | null;
  is_templated?: boolean | null;
  status: "active" | "draft" | "archived";
  runs: number;
  updated_at: string;
  average_score?: number | null;
  domain?: string | null;
  rubric?: ApiRubricCriterion[] | null;
  expected_output_format?: string | null;
};

type ApiDataset = {
  id: string;
  name: string;
  question_count: number;
  avg_score?: number | null;
  evaluations: number;
  last_used?: string | null;
  created_at?: string | null;
};

type ApiQuestion = {
  id: number;
  dataset_id: string;
  question_text: string;
  expected_answer?: string | null;
  question_order: number;
  variable_values?: Record<string, unknown> | null;
  tags?: string[] | null;
  difficulty?: string | null;
  case_type?: string | null;
};

type ApiDatasetWithQuestions = {
  dataset: ApiDataset;
  questions: Array<{
    id: number;
    dataset_id: string;
    question_text: string;
    expected_answer?: string | null;
    question_order: number;
    variable_values?: Record<string, unknown> | null;
    tags?: string[] | null;
    difficulty?: string | null;
    case_type?: string | null;
  }>;
};

type ApiDimensionScore = {
  score: number;
  reasoning: string;
};

type ApiEvaluation = {
  id: string;
  average_score: number;
  dataset: string;
  prompts: string[];
  created_at: string;
  per_prompt_scores?: Record<string, number> | null;
};

type ApiEvaluationWithDetails = ApiEvaluation & {
  total_items: number;
  scores: number[];
  details: Array<{
    prompt_id: string;
    question: string;
    response: string;
    score: number;
    strengths: string[] | null;
    weaknesses: string[] | null;
    dimension_scores?: Record<string, ApiDimensionScore> | null;
    judge_reasoning?: string | null;
    reference_used?: boolean | null;
  }>;
};

type ApiGenerateTestCases = {
  test_cases: GeneratedTestCase[];
};

// ── Mapper helpers ─────────────────────────────────────────────────────────────

function mapPrompt(p: ApiPrompt): PromptTemplate {
  return {
    id: p.id,
    name: p.name,
    content: p.template,
    variables: p.variables ?? undefined,
    isTemplated: p.is_templated ?? undefined,
    status: p.status,
    avgScore: p.average_score ?? undefined,
    runs: p.runs,
    updatedAt: p.updated_at,
    domain: p.domain ?? undefined,
    rubric: p.rubric
      ? p.rubric.map((r): RubricCriterion => ({ name: r.name, description: r.description, weight: r.weight }))
      : undefined,
    expectedOutputFormat: p.expected_output_format ?? undefined,
  };
}

function mapDataset(d: ApiDataset): DatasetItem {
  return {
    id: d.id,
    name: d.name,
    questions: d.question_count,
    avgScore: d.avg_score ?? undefined,
    evaluations: d.evaluations,
    lastUsed: d.last_used ?? "never",
    createdAt: d.created_at ?? undefined,
  };
}

function mapQuestion(q: ApiQuestion): DatasetQuestion {
  return {
    id: q.id,
    datasetId: q.dataset_id,
    questionText: q.question_text,
    expectedAnswer: q.expected_answer ?? undefined,
    questionOrder: q.question_order,
    variableValues: q.variable_values ?? undefined,
    tags: q.tags ?? undefined,
    difficulty: q.difficulty ?? undefined,
    caseType: q.case_type ?? undefined,
  };
}

// ── API client ─────────────────────────────────────────────────────────────────

export const api = {
  // ── Dashboard ──────────────────────────────────────────────────────────────

  getDashboardStats: async (): Promise<DashboardStats> => {
    const s = await request<ApiStats>("/stats");
    return {
      totalEvals: s.total_evaluations,
      activePrompts: s.active_prompts,
      avgScore: s.average_score,
      successRate: s.success_rate,
    };
  },

  getPerformanceTrend: async (): Promise<TrendPoint[]> => {
    const evals = await request<ApiEvaluation[]>("/evaluations");
    return evals.slice(0, 7).map((e, i) => ({
      date: e.created_at?.slice(0, 10) || `Run ${i + 1}`,
      score: e.average_score,
    }));
  },

  getRecentEvals: async (): Promise<EvalSummary[]> => {
    const evals = await request<ApiEvaluation[]>("/evaluations");
    return evals.map((e) => ({
      id: e.id,
      dataset: e.dataset,
      promptNames: e.prompts,
      winner: e.prompts[0] ?? "N/A",
      score: e.average_score,
      createdAt: e.created_at,
    }));
  },

  // ── Prompts ────────────────────────────────────────────────────────────────

  getTopPrompts: async (): Promise<PromptTemplate[]> => {
    const prompts = await request<ApiPrompt[]>("/prompts");
    return prompts.map(mapPrompt).sort((a, b) => (b.avgScore ?? 0) - (a.avgScore ?? 0));
  },

  getPrompts: async (): Promise<PromptTemplate[]> => {
    const prompts = await request<ApiPrompt[]>("/prompts");
    return prompts.map(mapPrompt);
  },

  getPrompt: async (id: string): Promise<PromptTemplate> => {
    const p = await request<ApiPrompt>(`/prompts/${encodeURIComponent(id)}`);
    return mapPrompt(p);
  },

  createPrompt: async (payload: {
    name: string;
    template: string;
    variables?: string[];
    is_templated?: boolean;
    status?: "active" | "draft" | "archived";
    domain?: string;
    rubric?: RubricCriterion[];
    expected_output_format?: string;
  }): Promise<PromptTemplate> => {
    const p = await requestWithBody<ApiPrompt>("/prompts", "POST", payload);
    return mapPrompt(p);
  },

  updatePrompt: async (
    id: string,
    payload: {
      name?: string;
      template?: string;
      status?: string;
      domain?: string;
      rubric?: RubricCriterion[];
      expected_output_format?: string;
    }
  ): Promise<PromptTemplate> => {
    const p = await requestWithBody<ApiPrompt>(`/prompts/${id}`, "PUT", payload);
    return mapPrompt(p);
  },

  deletePrompt: async (id: string): Promise<{ deleted: boolean; id: string }> =>
    requestWithBody(`/prompts/${id}`, "DELETE"),

  generatePrompt: async (description: string): Promise<GeneratedPrompt> => {
    const raw = await requestWithBody<{
      template: string;
      variables: string[];
      domain: string;
      rubric: ApiRubricCriterion[];
      expected_output_format: string;
    }>("/prompts/generate", "POST", { description });
    return {
      template: raw.template,
      variables: raw.variables,
      domain: raw.domain,
      rubric: raw.rubric.map((r) => ({ name: r.name, description: r.description, weight: r.weight })),
      expectedOutputFormat: raw.expected_output_format,
    };
  },

  // ── Datasets ───────────────────────────────────────────────────────────────

  getDatasets: async (): Promise<DatasetItem[]> => {
    const datasets = await request<ApiDataset[]>("/datasets");
    return datasets.map(mapDataset);
  },

  createDataset: async (payload: {
    name: string;
    question_count: number;
  }): Promise<DatasetItem> => {
    const d = await requestWithBody<ApiDataset>("/datasets", "POST", payload);
    return mapDataset(d);
  },

  createDatasetFromQuestions: async (payload: {
    name: string;
    description?: string;
    questions: Array<{
      question: string;
      answer?: string | null;
      variable_values?: Record<string, unknown>;
      tags?: string[];
      difficulty?: string;
      case_type?: string;
      reasoning?: string;
    }>;
  }): Promise<DatasetItem> => {
    const res = await requestWithBody<ApiDatasetWithQuestions>("/datasets", "POST", payload);
    return mapDataset(res.dataset);
  },

  getDataset: async (id: string): Promise<DatasetItem> => {
    const d = await request<ApiDataset>(`/datasets/${encodeURIComponent(id)}`);
    return mapDataset(d);
  },

  getDatasetQuestions: async (id: string): Promise<DatasetQuestion[]> => {
    const qs = await request<ApiQuestion[]>(`/datasets/${encodeURIComponent(id)}/questions`);
    return qs.map(mapQuestion);
  },

  updateDataset: async (id: string, payload: { name?: string }): Promise<DatasetItem> => {
    const d = await requestWithBody<ApiDataset>(`/datasets/${encodeURIComponent(id)}`, "PUT", payload);
    return mapDataset(d);
  },

  deleteDataset: async (id: string): Promise<{ deleted: boolean; id: string }> =>
    requestWithBody(`/datasets/${id}`, "DELETE"),

  // ── Test cases ─────────────────────────────────────────────────────────────

  generateTestCases: async (payload: {
    prompt_id: string;
    count?: number;
  }): Promise<GeneratedTestCase[]> => {
    const res = await requestWithBody<ApiGenerateTestCases>("/questions/generate", "POST", payload);
    return res.test_cases;
  },

  // ── Evaluations ────────────────────────────────────────────────────────────

  runEvaluation: async (payload: {
    dataset_id?: string;
    dataset_path?: string;
    prompt_ids: string[];
  }): Promise<ApiEvaluation> => requestWithBody("/evaluate", "POST", payload),

  getHistory: async () => api.getRecentEvals(),
  getResultsList: async () => api.getRecentEvals(),

  getResultDetail: async (id: string): Promise<ResultsDetail> => {
    const e = await request<ApiEvaluationWithDetails>(`/evaluations/${id}`);

    // Use server-computed per_prompt_scores when available, fall back to computing from details.
    const promptScores: { name: string; score: number }[] = e.prompts.map((pid) => {
      const fromServer = e.per_prompt_scores?.[pid];
      if (fromServer !== undefined) return { name: pid, score: fromServer };
      const rows = (e.details ?? []).filter((d) => d.prompt_id === pid);
      const score =
        rows.length > 0 ? rows.reduce((s, d) => s + (d.score ?? 0), 0) / rows.length : e.average_score;
      return { name: pid, score };
    });

    // Group details by question text, then build QuestionComparison entries.
    const byQuestion = new Map<string, ApiEvaluationWithDetails["details"]>();
    for (const d of e.details ?? []) {
      const list = byQuestion.get(d.question) ?? [];
      list.push(d);
      byQuestion.set(d.question, list);
    }

    const questions = Array.from(byQuestion.entries()).map(([question, rows]) => {
      const sorted = [...rows].sort(
        (a, b) => e.prompts.indexOf(a.prompt_id) - e.prompts.indexOf(b.prompt_id)
      );
      const a = sorted[0];
      const b = sorted[1];

      const mapDimScores = (
        raw?: Record<string, ApiDimensionScore> | null
      ): Record<string, DimensionScore> | undefined =>
        raw
          ? Object.fromEntries(
              Object.entries(raw).map(([k, v]) => [k, { score: v.score, reasoning: v.reasoning }])
            )
          : undefined;

      return {
        question,
        promptA: {
          name: a?.prompt_id ?? "—",
          score: a?.score ?? 0,
          response: a?.response ?? "",
          strengths: a?.strengths ?? [],
          weaknesses: a?.weaknesses ?? [],
          dimensionScores: mapDimScores(a?.dimension_scores),
          judgeReasoning: a?.judge_reasoning ?? undefined,
          referenceUsed: a?.reference_used ?? undefined,
        },
        promptB: b
          ? {
              name: b.prompt_id,
              score: b.score ?? 0,
              response: b.response ?? "",
              strengths: b.strengths ?? [],
              weaknesses: b.weaknesses ?? [],
              dimensionScores: mapDimScores(b.dimension_scores),
              judgeReasoning: b.judge_reasoning ?? undefined,
              referenceUsed: b.reference_used ?? undefined,
            }
          : undefined,
      };
    });

    return { id: e.id, dataset: e.dataset, promptScores, questions };
  },
};
