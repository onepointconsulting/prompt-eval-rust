use crate::models::{
    CreateDatasetRequest, Dataset, DatasetUploadRequest, DatasetWithQuestions, DeleteResponse,
    Question, QuestionInput,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::Value;
use serde_json::json;
use sqlx::PgPool;

// GET /api/datasets
pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Dataset>>, StatusCode> {
    println!("📋 Listing datasets from database");

    let datasets = sqlx::query_as::<_, Dataset>(
        r#"
        SELECT
            id,
            name,
            question_count,
            avg_score,
            evaluations,
            NULL::text as last_used,
            created_at::text as created_at
        FROM datasets
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(datasets))
}

// POST /api/datasets
pub async fn create(
    State(pool): State<PgPool>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    // Backward-compatible:
    // 1) {name, question_count}
    // 2) {name, description?, questions:[...]}
    if payload.get("questions").is_some() {
        let upload_payload: DatasetUploadRequest =
            serde_json::from_value(payload).map_err(|_| StatusCode::BAD_REQUEST)?;
        let created = create_dataset_with_questions(&pool, upload_payload).await?;
        return Ok((StatusCode::CREATED, Json(json!(created))));
    }

    let payload: CreateDatasetRequest =
        serde_json::from_value(payload).map_err(|_| StatusCode::BAD_REQUEST)?;
    println!("➕ Creating dataset: {}", payload.name);

    let id = format!("ds_{}", chrono::Utc::now().timestamp());
    let dataset = sqlx::query_as::<_, Dataset>(
        r#"
        INSERT INTO datasets (id, name, question_count, avg_score, evaluations, created_at)
        VALUES ($1, $2, $3, NULL, 0, NOW())
        RETURNING
            id,
            name,
            question_count,
            avg_score,
            evaluations,
            NULL::text as last_used,
            created_at::text as created_at
        "#,
    )
    .bind(id)
    .bind(payload.name)
    .bind(payload.question_count)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(json!(dataset))))
}

// POST /api/datasets/upload
pub async fn upload(
    State(pool): State<PgPool>,
    Json(payload): Json<DatasetUploadRequest>,
) -> Result<(StatusCode, Json<DatasetWithQuestions>), StatusCode> {
    let created = create_dataset_with_questions(&pool, payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

// GET /api/datasets/:id
pub async fn get(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<Dataset>, StatusCode> {
    println!("🔍 Getting dataset: {}", id);

    let dataset = sqlx::query_as::<_, Dataset>(
        r#"
        SELECT
            id,
            name,
            question_count,
            avg_score,
            evaluations,
            NULL::text as last_used,
            created_at::text as created_at
        FROM datasets
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(dataset))
}

// GET /api/datasets/:id/questions
pub async fn get_questions(
    State(pool): State<PgPool>,
    Path(dataset_id): Path<String>,
) -> Result<Json<Vec<Question>>, StatusCode> {
    println!("📋 Getting questions for dataset: {}", dataset_id);

    let questions = sqlx::query_as::<_, Question>(
        r#"
        SELECT
            id,
            dataset_id,
            question_text,
            expected_answer,
            COALESCE(question_order, 0) as question_order,
            variable_values,
            tags
        FROM questions
        WHERE dataset_id = $1
        ORDER BY question_order
        "#,
    )
    .bind(dataset_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(questions))
}

// POST /api/datasets/:id/questions
pub async fn add_question(
    State(pool): State<PgPool>,
    Path(dataset_id): Path<String>,
    Json(payload): Json<QuestionInput>,
) -> Result<(StatusCode, Json<Question>), StatusCode> {
    println!("➕ Adding question to dataset: {}", dataset_id);

    // Ensure dataset exists
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM datasets WHERE id = $1")
        .bind(&dataset_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Append ordering at end
    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(question_order), -1) + 1 FROM questions WHERE dataset_id = $1",
    )
    .bind(&dataset_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let created = sqlx::query_as::<_, Question>(
        r#"
        INSERT INTO questions (dataset_id, question_text, expected_answer, question_order, variable_values, tags)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id,
            dataset_id,
            question_text,
            expected_answer,
            question_order,
            variable_values,
            tags
        "#,
    )
    .bind(&dataset_id)
    .bind(payload.question)
    .bind(payload.answer)
    .bind(next_order)
    .bind(payload.variable_values)
    .bind(payload.tags)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Keep datasets.question_count in sync
    sqlx::query("UPDATE datasets SET question_count = question_count + 1 WHERE id = $1")
        .bind(&dataset_id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(created)))
}

// DELETE /api/datasets/:id
pub async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    println!("🗑️  Deleting dataset: {}", id);

    let result = sqlx::query("DELETE FROM datasets WHERE id = $1")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(DeleteResponse { deleted: true, id }))
}

async fn create_dataset_with_questions(
    pool: &PgPool,
    payload: DatasetUploadRequest,
) -> Result<DatasetWithQuestions, StatusCode> {
    println!("📤 Uploading dataset: {}", payload.name);
    if payload.questions.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let dataset_id = format!("ds_{}", chrono::Utc::now().timestamp());
    let question_count = payload.questions.len() as i32;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let dataset = sqlx::query_as::<_, Dataset>(
        r#"
        INSERT INTO datasets (id, name, description, question_count, avg_score, evaluations, created_at)
        VALUES ($1, $2, $3, $4, NULL, 0, NOW())
        RETURNING
            id,
            name,
            question_count,
            avg_score,
            evaluations,
            NULL::text as last_used,
            created_at::text as created_at
        "#
    )
    .bind(dataset_id)
    .bind(payload.name)
    .bind(payload.description)
    .bind(question_count)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut created_questions: Vec<Question> = Vec::new();
    for (idx, q) in payload.questions.iter().enumerate() {
        let question = sqlx::query_as::<_, Question>(
            r#"
            INSERT INTO questions (dataset_id, question_text, expected_answer, question_order, variable_values, tags)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id,
                dataset_id,
                question_text,
                expected_answer,
                question_order,
                variable_values,
                tags
            "#,
        )
        .bind(&dataset.id)
        .bind(&q.question)
        .bind(&q.answer)
        .bind(idx as i32)
        .bind(&q.variable_values)
        .bind(&q.tags)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        created_questions.push(question);
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("✅ Uploaded {} questions", question_count);
    Ok(DatasetWithQuestions {
        dataset,
        questions: created_questions,
    })
}
