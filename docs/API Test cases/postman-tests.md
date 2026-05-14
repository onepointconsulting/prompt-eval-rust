# Postman Test Guide

Base URL: `http://127.0.0.1:3001`

Run requests in the order listed. Each section depends on IDs from the section before it.
Copy IDs from responses and substitute them where you see `{{prompt_id}}`, `{{dataset_id}}`, etc.

---

## 1. Prompt Generation & Management

### 1.1 Generate a prompt template (AI)

```
POST /api/prompts/generate
Content-Type: application/json
```

```json
{
  "description": "A teaching assistant for beginner programmers learning Python. It should guide students with the Socratic method — asking one question at a time — and adapt to their emotional state."
}
```

**Check response:**
- `template` — contains `{{DOUBLE_BRACE}}` placeholders
- `variables` — array of placeholder names found in the template
- `domain` — short snake_case label e.g. `"educational_assistant"`
- `rubric` — array of 3-5 criteria, each with `name`, `description`, `weight`
- Weights in rubric sum to `1.0`
- `expected_output_format` — describes what a good response looks like

---

### 1.2 Create a prompt manually (no rubric)

```
POST /api/prompts
Content-Type: application/json
```

```json
{
  "name": "Basic Python Tutor",
  "template": "You are a helpful Python tutor.\n\nStudent question: {{QUESTION}}\n\nProvide a clear, beginner-friendly answer.",
  "status": "active"
}
```

**Check response:**
- `id` — copy this as `{{prompt_id_basic}}`
- `variables` — should auto-extract `["QUESTION"]`
- `is_templated` — should be `true`
- `domain` — `null` (none provided)
- `rubric` — `null` (none provided — judge will use generic fallback)

---

### 1.3 Create a prompt with rubric (full payload)

Use the output from **1.1** — paste the generated `template`, `rubric`, `domain`, and `expected_output_format` here.

```
POST /api/prompts
Content-Type: application/json
```

```json
{
  "name": "Socratic Python Tutor",
  "template": "You are a Socratic teaching mentor for beginner programmers.\n\nStudent profile: {{STUDENT_PROFILE}}\nStudent message: {{STUDENT_MESSAGE}}\n\nGuide the student by asking exactly one clarifying question. Do not reveal the answer directly.",
  "status": "active",
  "domain": "educational_assistant",
  "rubric": [
    {"name": "socratic_adherence", "description": "Did it ask exactly one question without revealing the answer?", "weight": 0.4},
    {"name": "emotional_awareness", "description": "Did it acknowledge emotional state if signals were present?", "weight": 0.3},
    {"name": "clarity", "description": "Was the question clear and appropriately pitched for a beginner?", "weight": 0.3}
  ],
  "expected_output_format": "A single conversational question in plain English, 1-2 sentences."
}
```

**Check response:**
- `id` — copy this as `{{prompt_id_socratic}}`
- `rubric` — the JSON array should be stored and returned as-is
- `domain` — `"educational_assistant"`

---

### 1.4 List all prompts

```
GET /api/prompts
```

**Check response:**
- Array contains the two prompts created above
- Both have `average_score: null` and `runs: 0` (not yet evaluated)
- `domain` and `rubric` populated on the Socratic prompt, null on the Basic prompt

---

### 1.5 Get a single prompt

```
GET /api/prompts/{{prompt_id_socratic}}
```

**Check response:** Full prompt with rubric array intact.

---

### 1.6 Update a prompt

```
PUT /api/prompts/{{prompt_id_basic}}
Content-Type: application/json
```

```json
{
  "status": "active",
  "domain": "code_tutoring",
  "rubric": [
    {"name": "relevance", "description": "Does it address what the student asked?", "weight": 0.4},
    {"name": "clarity", "description": "Is the explanation beginner-friendly?", "weight": 0.4},
    {"name": "accuracy", "description": "Is the information factually correct?", "weight": 0.2}
  ]
}
```

**Check response:**
- `status` → `"active"`
- `rubric` updated, `name` and `template` unchanged (patch behaviour)

---

## 2. Dataset & Question Management

### 2.1 Create a dataset with questions (upload)

This is the most realistic test case — includes `expected_answer`, `difficulty`, and `case_type` on questions.

```
POST /api/datasets/upload
Content-Type: application/json
```

```json
{
  "name": "Python Tutor Test Set",
  "description": "Test cases for evaluating Python teaching assistants",
  "questions": [
    {
      "question": "I don't understand why my for loop isn't working. Here's my code: for i in range(10) print(i)",
      "answer": "Must point out the missing colon after range(10) by asking a guiding question, not stating it directly. Should not show corrected code.",
      "variable_values": {
        "STUDENT_PROFILE": "{\"name\": \"Ali\", \"level\": \"beginner\", \"language\": \"English\"}",
        "STUDENT_MESSAGE": "I don't understand why my for loop isn't working. for i in range(10) print(i)"
      },
      "difficulty": "easy",
      "case_type": "happy_path",
      "tags": ["topic:syntax", "difficulty:easy"],
      "reasoning": "Basic syntax error case — tests whether the tutor guides rather than tells."
    },
    {
      "question": "I've been trying for 3 hours and I can't figure out list comprehensions. I'm so stupid.",
      "answer": "Must acknowledge emotional distress before engaging technically. Should not call the student stupid or confirm the self-criticism. Should ask one question about what they've tried.",
      "variable_values": {
        "STUDENT_PROFILE": "{\"name\": \"Sara\", \"level\": \"beginner\", \"language\": \"English\", \"emotional_state\": \"frustrated\"}",
        "STUDENT_MESSAGE": "I've been trying for 3 hours and I can't figure out list comprehensions. I'm so stupid."
      },
      "difficulty": "medium",
      "case_type": "emotional_stress",
      "tags": ["topic:list_comprehension", "difficulty:medium", "dimension_tested:emotional_awareness"],
      "reasoning": "Tests emotional awareness — the primary failure mode for tutors that jump straight to technical content."
    },
    {
      "question": "Just write me the complete solution for my homework assignment on recursion.",
      "answer": "Must decline to write the solution directly. Should redirect with a question about what the student already understands about recursion.",
      "variable_values": {
        "STUDENT_PROFILE": "{\"name\": \"Omar\", \"level\": \"beginner\", \"language\": \"English\"}",
        "STUDENT_MESSAGE": "Just write me the complete solution for my homework assignment on recursion."
      },
      "difficulty": "adversarial",
      "case_type": "adversarial",
      "tags": ["topic:recursion", "difficulty:adversarial", "dimension_tested:socratic_adherence"],
      "reasoning": "Adversarial case — tests whether the prompt correctly refuses to do homework while staying constructive."
    }
  ]
}
```

**Check response:**
- `dataset.id` — copy as `{{dataset_id}}`
- `questions` — array of 3 questions with `id`, `expected_answer`, `difficulty`, `case_type` all present

---

### 2.2 List datasets

```
GET /api/datasets
```

**Check:** Dataset appears with `question_count: 3`.

---

### 2.3 Get dataset details

```
GET /api/datasets/{{dataset_id}}
```

---

### 2.4 Get questions for dataset

```
GET /api/datasets/{{dataset_id}}/questions
```

**Check response:**
- All 3 questions present
- `expected_answer` populated
- `difficulty` values: `easy`, `medium`, `adversarial`
- `case_type` values: `happy_path`, `emotional_stress`, `adversarial`

---

### 2.5 Add a single question

```
POST /api/datasets/{{dataset_id}}/questions
Content-Type: application/json
```

```json
{
  "question": "What's the difference between a list and a tuple in Python?",
  "answer": "Should ask what the student already knows or has tried, not just explain the difference directly.",
  "variable_values": {
    "STUDENT_PROFILE": "{\"name\": \"Test\", \"level\": \"beginner\", \"language\": \"English\"}",
    "STUDENT_MESSAGE": "What's the difference between a list and a tuple in Python?"
  },
  "difficulty": "easy",
  "case_type": "happy_path",
  "tags": ["topic:data_types", "difficulty:easy"]
}
```

**Check response:**
- `question_order` — should be 3 (appended at end)

---

## 3. Generate Test Cases (AI)

### 3.1 Generate test cases for a prompt

```
POST /api/questions/generate
Content-Type: application/json
```

```json
{
  "prompt_id": "{{prompt_id_socratic}}",
  "count": 6
}
```

**Check response:**
- `test_cases` — array of 6 items
- Each item has: `variable_values`, `expected_answer`, `difficulty`, `case_type`, `tags`, `reasoning`
- `expected_answer` should be a semantic specification (not a verbatim answer)
- `difficulty` distribution should roughly follow: 2 easy, 2 medium, 1 hard, 1 adversarial
- Tags should reference rubric dimensions e.g. `"dimension_tested:socratic_adherence"`

---

### 3.2 Generate test cases for basic prompt (no rubric)

```
POST /api/questions/generate
Content-Type: application/json
```

```json
{
  "prompt_id": "{{prompt_id_basic}}",
  "count": 4
}
```

**Check response:**
- Should still work — falls back to generic domain + criteria
- `difficulty` and `case_type` still populated

---

## 4. Run Evaluations

### 4.1 Evaluate one prompt (baseline check)

```
POST /api/evaluate
Content-Type: application/json
```

```json
{
  "dataset_id": "{{dataset_id}}",
  "prompt_ids": ["{{prompt_id_basic}}"]
}
```

**This will make real API calls — takes 30–60 seconds for 3 questions.**

**Check response:**
- `id` — copy as `{{run_id_basic}}`
- `average_score` — 1.0–10.0
- `per_prompt_scores` — `{"{{prompt_id_basic}}": <score>}` (single entry)
- `scores` — array of 3 individual scores

**Check DB (via psql or a DB client):**
```sql
SELECT prompt_id, score, strengths, weaknesses, judge_reasoning, reference_used, dimension_scores
FROM evaluation_details
WHERE run_id = '{{run_id_basic}}';
```
- `strengths` and `weaknesses` should be populated (not empty)
- `judge_reasoning` should have 2-3 sentences of reasoning
- `reference_used` should be `true` for questions that had `expected_answer`
- `dimension_scores` should be a JSON object with per-criterion scores

---

### 4.2 A/B evaluation — compare two prompts

```
POST /api/evaluate
Content-Type: application/json
```

```json
{
  "dataset_id": "{{dataset_id}}",
  "prompt_ids": ["{{prompt_id_basic}}", "{{prompt_id_socratic}}"]
}
```

**This makes 6 API calls (3 questions × 2 prompts) — takes 60–120 seconds.**

**Check response:**
- `id` — copy as `{{run_id_ab}}`
- `per_prompt_scores` — two entries, one per prompt e.g.:
  ```json
  {
    "p_111": 6.2,
    "p_222": 8.1
  }
  ```
- The Socratic prompt (with rubric + domain-specific criteria) should score higher

**Check prompt stats updated:**
```sql
SELECT id, name, runs, average_score, total_score_sum, total_score_count
FROM prompts
WHERE id IN ('{{prompt_id_basic}}', '{{prompt_id_socratic}}');
```
- `runs` incremented to 1 (or 2 if you ran 4.1 first)
- `average_score` = `total_score_sum / total_score_count`
- `total_score_count` = number of questions × number of runs for this prompt

---

### 4.3 Evaluate using dataset_path (legacy fallback)

```
POST /api/evaluate
Content-Type: application/json
```

```json
{
  "dataset_path": "Python Tutor Test Set",
  "prompt_ids": ["{{prompt_id_basic}}"]
}
```

**Check:** Should resolve the dataset by name and run successfully.

---

## 5. View Results

### 5.1 List all evaluations

```
GET /api/evaluations
```

**Check response:**
- Both runs from section 4 appear
- Each has `per_prompt_scores` populated
- `scores` array is present

---

### 5.2 Get evaluation details

```
GET /api/evaluations/{{run_id_ab}}
```

**Check response:**
- `per_prompt_scores` — both prompt scores present
- `details` — 6 entries (3 questions × 2 prompts)
- Each detail has:
  - `strengths` — non-empty array
  - `weaknesses` — non-empty array
  - `dimension_scores` — per-criterion breakdown
  - `judge_reasoning` — 2-3 sentence assessment
  - `reference_used` — `true` for questions with `expected_answer`

---

## 6. Dashboard Stats

```
GET /api/stats
```

> ⚠️ This still returns hardcoded values (`124`, `12`, `8.5`, `95`).
> Stats handler is the next item on the implementation list.

---

## 7. Cleanup (optional)

### Delete a prompt

```
DELETE /api/prompts/{{prompt_id_basic}}
```

**Check:** `{"deleted": true, "id": "..."}` and status 200.

---

### Delete a dataset

```
DELETE /api/datasets/{{dataset_id}}
```

**Check:** Cascades — also deletes all questions in the dataset.
Verify in DB: `SELECT COUNT(*) FROM questions WHERE dataset_id = '{{dataset_id}}';` → 0.

---

## Key things to verify end-to-end

| What | Where to check | Expected |
|---|---|---|
| Rubric saved on prompt | GET /api/prompts/:id | `rubric` array present |
| Judge uses rubric | evaluation_details.dimension_scores | Keys match rubric criterion names |
| expected_answer used | evaluation_details.reference_used | `true` for questions with expected_answer |
| Strengths populated | evaluation_details.strengths | Non-empty array, not just `[]` |
| Weaknesses populated | evaluation_details.weaknesses | Non-empty array |
| Rolling mean correct | prompts.average_score | = total_score_sum / total_score_count |
| Per-prompt scores | evaluation_runs.per_prompt_scores | One entry per prompt_id in the run |
| Socratic prompt scores higher | per_prompt_scores comparison | Rubric-specific prompt > generic prompt |
