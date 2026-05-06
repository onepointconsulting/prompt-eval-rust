import type {
  DashboardStats,
  DatasetItem,
  EvalSummary,
  GeneratedPrompt,
  GeneratedTestCase,
  PromptTemplate,
  ResultsDetail,
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

type ApiStats = {
  total_evaluations: number;
  active_prompts: number;
  average_score: number;
  success_rate: number;
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
};
type ApiDataset = {
  id: string;
  name: string;
  question_count: number;
  avg_score?: number | null;
  evaluations: number;
  last_used?: string | null;
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
  }>;
};
type ApiEvaluation = {
  id: string;
  average_score: number;
  dataset: string;
  prompts: string[];
  created_at: string;
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
  }>;
};
type ApiGenerateTestCases = {
  test_cases: GeneratedTestCase[];
};

export const api = {
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
  getTopPrompts: async (): Promise<PromptTemplate[]> => {
    const prompts = await request<ApiPrompt[]>("/prompts");
    return prompts
      .map((p) => ({
        id: p.id,
        name: p.name,
        content: p.template,
        variables: p.variables ?? undefined,
        isTemplated: p.is_templated ?? undefined,
        status: p.status,
        avgScore: p.average_score ?? undefined,
        runs: p.runs,
        updatedAt: p.updated_at,
      }))
      .sort((a, b) => (b.avgScore ?? 0) - (a.avgScore ?? 0));
  },
  getDatasets: async (): Promise<DatasetItem[]> => {
    const datasets = await request<ApiDataset[]>("/datasets");
    return datasets.map((d) => ({
      id: d.id,
      name: d.name,
      questions: d.question_count,
      avgScore: d.avg_score ?? undefined,
      evaluations: d.evaluations,
      lastUsed: d.last_used ?? "never",
    }));
  },
  createDataset: async (payload: {
    name: string;
    question_count: number;
  }): Promise<DatasetItem> => {
    const d = await requestWithBody<ApiDataset>("/datasets", "POST", payload);
    return {
      id: d.id,
      name: d.name,
      questions: d.question_count,
      avgScore: d.avg_score ?? undefined,
      evaluations: d.evaluations,
      lastUsed: d.last_used ?? "never",
    };
  },
  createDatasetFromQuestions: async (payload: {
    name: string;
    description?: string;
    questions: Array<{
      question: string;
      answer?: string | null;
      variable_values?: Record<string, unknown>;
      tags?: string[];
    }>;
  }): Promise<DatasetItem> => {
    const res = await requestWithBody<ApiDatasetWithQuestions>("/datasets", "POST", payload);
    const d = res.dataset;
    return {
      id: d.id,
      name: d.name,
      questions: d.question_count,
      avgScore: d.avg_score ?? undefined,
      evaluations: d.evaluations,
      lastUsed: d.last_used ?? "never",
    };
  },
  deleteDataset: async (id: string): Promise<{ deleted: boolean; id: string }> =>
    requestWithBody(`/datasets/${id}`, "DELETE"),
  getPrompts: async (): Promise<PromptTemplate[]> => {
    const prompts = await request<ApiPrompt[]>("/prompts");
    return prompts.map((p) => ({
      id: p.id,
      name: p.name,
      content: p.template,
      variables: p.variables ?? undefined,
      isTemplated: p.is_templated ?? undefined,
      status: p.status,
      avgScore: p.average_score ?? undefined,
      runs: p.runs,
      updatedAt: p.updated_at,
    }));
  },
  getPrompt: async (id: string): Promise<PromptTemplate> => {
    const p = await request<ApiPrompt>(`/prompts/${encodeURIComponent(id)}`);
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
    };
  },
  createPrompt: async (payload: {
    name: string;
    template: string;
    variables?: string[];
    is_templated?: boolean;
    status?: "active" | "draft" | "archived";
  }): Promise<PromptTemplate> => {
    const p = await requestWithBody<ApiPrompt>("/prompts", "POST", payload);
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
    };
  },
  updatePrompt: async (
    id: string,
    payload: { name?: string; template?: string; status?: string }
  ): Promise<PromptTemplate> => {
    const p = await requestWithBody<ApiPrompt>(`/prompts/${id}`, "PUT", payload);
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
    };
  },
  generatePrompt: async (description: string): Promise<GeneratedPrompt> =>
    requestWithBody("/prompts/generate", "POST", { description }),
  generateTestCases: async (payload: {
    prompt_id: string;
    count?: number;
  }): Promise<GeneratedTestCase[]> => {
    const res = await requestWithBody<ApiGenerateTestCases>("/questions/generate", "POST", payload);
    return res.test_cases;
  },
  deletePrompt: async (id: string): Promise<{ deleted: boolean; id: string }> =>
    requestWithBody(`/prompts/${id}`, "DELETE"),
  getHistory: async () => api.getRecentEvals(),
  getResultsList: async () => api.getRecentEvals(),
  runEvaluation: async (payload: {
    dataset_id?: string;
    /** @deprecated Prefer dataset_id — still accepted for older clients */
    dataset_path?: string;
    prompt_ids: string[];
  }): Promise<ApiEvaluation> => requestWithBody("/evaluate", "POST", payload),
  getResultDetail: async (id: string): Promise<ResultsDetail> => {
    const e = await request<ApiEvaluationWithDetails>(`/evaluations/${id}`);

    const parseClaudeJson = (raw: string) => {
      try {
        return JSON.parse(raw) as unknown;
      } catch {
        return null;
      }
    };
    const extractAssistantText = (raw: string) => {
      const j = parseClaudeJson(raw);
      if (!j || typeof j !== "object") return raw;
      const content = (j as { content?: unknown }).content;
      if (!Array.isArray(content)) return raw;
      const firstText = content.find(
        (c): c is { type: "text"; text: string } =>
          typeof c === "object" &&
          c !== null &&
          (c as { type?: unknown }).type === "text" &&
          typeof (c as { text?: unknown }).text === "string"
      );
      const text = firstText?.text;
      return typeof text === "string" && text.trim().length ? text : raw;
    };
    const extractMeta = (raw: string): Record<string, string | number | boolean | null> => {
      const j = parseClaudeJson(raw);
      if (!j || typeof j !== "object") return {};
      const usage = (j as { usage?: unknown }).usage;
      const usageObj = usage && typeof usage === "object" ? (usage as Record<string, unknown>) : {};
      return {
        model: ((j as { model?: unknown }).model as string | undefined) ?? null,
        message_id: ((j as { id?: unknown }).id as string | undefined) ?? null,
        stop_reason: ((j as { stop_reason?: unknown }).stop_reason as string | undefined) ?? null,
        input_tokens: (usageObj.input_tokens as number | undefined) ?? null,
        output_tokens: (usageObj.output_tokens as number | undefined) ?? null,
        service_tier:
          (usageObj.service_tier as string | undefined) ??
          (((j as { service_tier?: unknown }).service_tier as string | undefined) ?? null),
      };
    };

    // Compute per-prompt averages from details (better than duplicating overall average_score).
    const byPrompt = new Map<string, { sum: number; n: number }>();
    for (const d of e.details ?? []) {
      const curr = byPrompt.get(d.prompt_id) ?? { sum: 0, n: 0 };
      curr.sum += d.score ?? 0;
      curr.n += 1;
      byPrompt.set(d.prompt_id, curr);
    }
    const promptScores = e.prompts.map((pid) => {
      const agg = byPrompt.get(pid);
      const score = agg && agg.n > 0 ? agg.sum / agg.n : e.average_score;
      return { name: pid, score };
    });

    // Group details by question, and map into the UI's QuestionDetail structure.
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
      return {
        question,
        promptA: {
          name: a?.prompt_id ?? "—",
          score: a?.score ?? 0,
          response: extractAssistantText(a?.response ?? ""),
          strengths: a?.strengths ?? [],
          weaknesses: a?.weaknesses ?? [],
          meta: extractMeta(a?.response ?? ""),
        },
        promptB: b
          ? {
              name: b.prompt_id,
              score: b.score ?? 0,
              response: extractAssistantText(b.response ?? ""),
              strengths: b.strengths ?? [],
              weaknesses: b.weaknesses ?? [],
              meta: extractMeta(b.response ?? ""),
            }
          : undefined,
      };
    });

    return {
      id: e.id,
      dataset: e.dataset,
      promptScores,
      questions,
    };
  },
};
