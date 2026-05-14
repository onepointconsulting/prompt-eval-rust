"use client";

import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { PageContainer } from "@/components/layout/PageContainer";
import { api } from "@/lib/api";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";

type QuestionRow = {
  question: string;
  answer: string;
  difficulty: string;
  case_type: string;
};

const DIFFICULTY_OPTIONS = ["easy", "medium", "hard", "adversarial"];
const CASE_TYPE_OPTIONS = [
  "happy_path",
  "edge_case",
  "adversarial",
  "emotional_stress",
  "multi_part",
  "out_of_scope",
];

export default function UploadDatasetPage() {
  const router = useRouter();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [questions, setQuestions] = useState<QuestionRow[]>([
    { question: "", answer: "", difficulty: "", case_type: "" },
  ]);
  const [submitting, setSubmitting] = useState(false);

  const addQuestion = () => {
    setQuestions((prev) => [...prev, { question: "", answer: "", difficulty: "", case_type: "" }]);
  };

  const removeQuestion = (index: number) => {
    setQuestions((prev) => prev.filter((_, i) => i !== index));
  };

  const updateQuestion = (index: number, field: keyof QuestionRow, value: string) => {
    setQuestions((prev) => {
      const next = [...prev];
      next[index] = { ...next[index], [field]: value };
      return next;
    });
  };

  const handleSubmit = async () => {
    if (!name.trim()) {
      toast.error("Dataset name is required.");
      return;
    }
    const cleaned = questions
      .map((q) => ({
        question: q.question.trim(),
        answer: q.answer.trim() || null,
        difficulty: q.difficulty || undefined,
        case_type: q.case_type || undefined,
      }))
      .filter((q) => q.question.length > 0);

    if (cleaned.length === 0) {
      toast.error("Add at least one question.");
      return;
    }

    setSubmitting(true);
    try {
      const created = await api.createDatasetFromQuestions({
        name: name.trim(),
        description: description.trim() || undefined,
        questions: cleaned,
      });
      toast.success(`Dataset "${created.name}" uploaded with ${created.questions} questions.`);
      router.push("/datasets");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Upload failed.");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <PageContainer
      title="Upload Dataset"
      description="Create a dataset and insert all questions in one request."
    >
      <Card className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-2">
          <Input
            placeholder="Dataset name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Input
            placeholder="Description (optional)"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>

        <div className="space-y-3">
          {questions.map((q, idx) => (
            <div key={idx} className="rounded-lg border p-3 space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-slate-600">Question {idx + 1}</span>
                {questions.length > 1 && (
                  <button
                    type="button"
                    onClick={() => removeQuestion(idx)}
                    className="text-xs text-red-500 hover:text-red-700"
                  >
                    Remove
                  </button>
                )}
              </div>
              <Input
                placeholder="Question text"
                value={q.question}
                onChange={(e) => updateQuestion(idx, "question", e.target.value)}
              />
              <Input
                placeholder="Expected answer / semantic spec (optional)"
                value={q.answer}
                onChange={(e) => updateQuestion(idx, "answer", e.target.value)}
              />
              <div className="grid grid-cols-2 gap-2">
                <Select
                  value={q.difficulty}
                  onChange={(e) => updateQuestion(idx, "difficulty", e.target.value)}
                >
                  <option value="">Difficulty (optional)</option>
                  {DIFFICULTY_OPTIONS.map((d) => (
                    <option key={d} value={d}>{d}</option>
                  ))}
                </Select>
                <Select
                  value={q.case_type}
                  onChange={(e) => updateQuestion(idx, "case_type", e.target.value)}
                >
                  <option value="">Case type (optional)</option>
                  {CASE_TYPE_OPTIONS.map((t) => (
                    <option key={t} value={t}>{t}</option>
                  ))}
                </Select>
              </div>
            </div>
          ))}
        </div>

        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={addQuestion} type="button">
            + Add Question
          </Button>
          <Button onClick={handleSubmit} disabled={submitting} type="button">
            {submitting ? "Uploading..." : "Upload Dataset"}
          </Button>
        </div>
      </Card>
    </PageContainer>
  );
}
