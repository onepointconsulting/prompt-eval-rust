import type {
  EvalDatasetItem,
  EvalRunItem,
  PromptVariant,
  ScoreSummary,
} from "@/types/eval";

export const promptVariants: PromptVariant[] = [
  {
    id: "v1",
    name: "Prompt v1",
    status: "baseline",
    template: "Please answer the user's question:\n\n{question}",
  },
  {
    id: "v2",
    name: "Prompt v2",
    status: "candidate",
    template:
      "Please answer the user's question with ample detail:\n\n{question}",
  },
];

export const datasetItems: EvalDatasetItem[] = [
  { id: 1, question: "What is the capital of Afghanistan?" },
  { id: 2, question: "In one sentence, what does HTTP stand for?" },
  { id: 3, question: "What is 15% of 80?" },
  { id: 4, question: "What is ownership in Rust?" },
  { id: 5, question: "Name two greenhouse gases." },
];

export const evalRunItems: EvalRunItem[] = [
  {
    id: 1,
    question: "What is the capital of Afghanistan?",
    response: "The capital of Afghanistan is Kabul.",
    score: 9.0,
  },
  {
    id: 2,
    question: "In one sentence, what does HTTP stand for?",
    response:
      "HTTP stands for Hypertext Transfer Protocol, a web protocol used to transfer hypermedia and API data.",
    score: 10.0,
  },
  {
    id: 3,
    question: "What is 15% of 80?",
    response: "15% of 80 is 12.",
    score: 8.0,
  },
  {
    id: 4,
    question: "What is ownership in Rust?",
    response:
      "Ownership is Rust's memory model where each value has one owner and is dropped when the owner goes out of scope.",
    score: 7.0,
  },
  {
    id: 5,
    question: "Name two greenhouse gases.",
    response:
      "Two greenhouse gases are carbon dioxide (CO2) and methane (CH4).",
    score: 9.0,
  },
];

const scores = evalRunItems.map((item) => item.score);

export const scoreSummary: ScoreSummary = {
  promptName: "Prompt v2",
  average: scores.reduce((sum, score) => sum + score, 0) / scores.length,
  min: Math.min(...scores),
  max: Math.max(...scores),
};
