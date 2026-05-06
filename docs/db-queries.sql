-- Datasets metadata
CREATE TABLE datasets (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    question_count INTEGER NOT NULL,
    avg_score DOUBLE PRECISION,
    evaluations INTEGER DEFAULT 0,
    created_by VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Questions belonging to datasets
CREATE TABLE questions (
    id SERIAL PRIMARY KEY,
    dataset_id VARCHAR(50) NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    question_text TEXT NOT NULL,
    expected_answer TEXT,
    question_order INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
    variable_values JSONB,
    tags TEXT[]
);

-- Evaluation runs
CREATE TABLE evaluation_runs (
    id VARCHAR(50) PRIMARY KEY,
    dataset_id VARCHAR(50) REFERENCES datasets(id),
    prompt_ids TEXT[] NOT NULL,
    average_score DOUBLE PRECISION NOT NULL,
    total_questions INTEGER NOT NULL,
    status VARCHAR(50) DEFAULT 'completed',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Detailed results per question
CREATE TABLE evaluation_details (
    id SERIAL PRIMARY KEY,
    run_id VARCHAR(50) REFERENCES evaluation_runs(id) ON DELETE CASCADE,
    question_id INTEGER REFERENCES questions(id),
    prompt_id VARCHAR(50),
    model_answer TEXT,
    score DOUBLE PRECISION,
    strengths TEXT[],
    weaknesses TEXT[],
    created_at TIMESTAMPTZ DEFAULT NOW()
);


-- Create prompts table
CREATE TABLE prompts (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    template TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    runs INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    average_score DOUBLE PRECISION
    variables TEXT[],
    is_templated BOOLEAN DEFAULT false
);


-- Create index for faster queries by status
CREATE INDEX idx_prompts_status ON prompts(status);
CREATE INDEX idx_prompts_updated ON prompts(updated_at DESC);

-- Insert some sample data for testing
INSERT INTO prompts (id, name, template, status, runs, updated_at, average_score) VALUES
('p_sample1', 'Basic Prompt', 'Please answer the user''s question:\n\n{question}', 'active', 0, NOW(), NULL),
('p_sample2', 'Professional Prompt', 'Please answer the user''s question, clearly and concisely, keep it short, professional, and to the point:\n\n{question}', 'active', 0, NOW(), NULL);


-- Indexes for performance
CREATE INDEX idx_questions_dataset ON questions(dataset_id);
CREATE INDEX idx_evaluation_runs_dataset ON evaluation_runs(dataset_id);
CREATE INDEX idx_evaluation_details_run ON evaluation_details(run_id);



