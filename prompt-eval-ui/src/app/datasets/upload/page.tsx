"use client";

import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Input";
import { PageContainer } from "@/components/layout/PageContainer";
import { useState } from "react";
import { toast } from "sonner";

type QuestionRow = {
  question: string;
  answer: string;
};

const API_BASE = "http://127.0.0.1:3001/api";

export default function UploadDatasetPage() {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [questions, setQuestions] = useState<QuestionRow[]>([
    { question: "", answer: "" },
  ]);
  const [submitting, setSubmitting] = useState(false);

  const addQuestion = () => {
    setQuestions((prev) => [...prev, { question: "", answer: "" }]);
  };

  const updateQuestion = (
    index: number,
    field: keyof QuestionRow,
    value: string
  ) => {
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
      }))
      .filter((q) => q.question.length > 0);

    if (cleaned.length === 0) {
      toast.error("Add at least one question.");
      return;
    }

    setSubmitting(true);
    try {
      const res = await fetch(`${API_BASE}/datasets/upload`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: name.trim(),
          description: description.trim() || null,
          questions: cleaned,
        }),
      });

      if (!res.ok) {
        throw new Error(`Upload failed (${res.status})`);
      }

      await res.json();
      toast.success("Dataset uploaded.");
      setName("");
      setDescription("");
      setQuestions([{ question: "", answer: "" }]);
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

        {questions.map((q, idx) => (
          <div key={idx} className="rounded-lg border p-3">
            <Input
              placeholder={`Question ${idx + 1}`}
              value={q.question}
              onChange={(e) => updateQuestion(idx, "question", e.target.value)}
            />
            <Input
              className="mt-2"
              placeholder="Expected answer (optional)"
              value={q.answer}
              onChange={(e) => updateQuestion(idx, "answer", e.target.value)}
            />
          </div>
        ))}

        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={addQuestion}>
            + Add Question
          </Button>
          <Button onClick={handleSubmit} disabled={submitting}>
            {submitting ? "Uploading..." : "Upload Dataset"}
          </Button>
        </div>
      </Card>
    </PageContainer>
  );
}
