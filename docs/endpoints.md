# API Endpoints Needed for Full UI

Total endpoints: **12**

## Evaluations (3)

- `POST /api/evaluate` - Run a new evaluation
- `GET /api/evaluations` - List all evaluations
- `GET /api/evaluations/:id` - Get specific evaluation details

## Datasets (4)

- `GET /api/datasets` - List all datasets
- `POST /api/datasets` - Upload a new dataset
- `GET /api/datasets/:id` - Get dataset details
- `DELETE /api/datasets/:id` - Delete a dataset

## Prompts (4)

- `GET /api/prompts` - List all prompts
- `POST /api/prompts` - Create a new prompt
- `PUT /api/prompts/:id` - Update a prompt
- `DELETE /api/prompts/:id` - Delete a prompt

## Stats (1)

- `GET /api/stats` - Dashboard statistics
