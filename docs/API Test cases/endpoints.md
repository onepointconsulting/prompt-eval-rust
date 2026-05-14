# API Endpoints

Total endpoints: **18**

## Evaluations (3)

- `POST /api/evaluate` - Run a new evaluation
- `GET /api/evaluations` - List evaluations
- `GET /api/evaluations/:id` - Get evaluation details

## Datasets (7)

- `GET /api/datasets` - List datasets
- `POST /api/datasets` - Create dataset (supports `question_count` or `questions[]`)
- `POST /api/datasets/upload` - Upload dataset with questions
- `GET /api/datasets/:id` - Get dataset details
- `DELETE /api/datasets/:id` - Delete dataset
- `GET /api/datasets/:id/questions` - List dataset questions
- `POST /api/datasets/:id/questions` - Add a question to dataset

## Prompts & Generation (7)

- `GET /api/prompts` - List prompts
- `POST /api/prompts` - Create prompt
- `POST /api/prompts/generate` - Generate prompt template from description
- `GET /api/prompts/:id` - Get prompt details
- `PUT /api/prompts/:id` - Update prompt
- `DELETE /api/prompts/:id` - Delete prompt
- `POST /api/questions/generate` - Generate test cases for a prompt

## Stats (1)

- `GET /api/stats` - Dashboard statistics
