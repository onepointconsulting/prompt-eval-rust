use crate::auth::AuthUser;
use crate::models::{
    CreateDatasetRequest, Dataset, DatasetUploadRequest, DatasetWithQuestions, DeleteResponse,
    Question, QuestionInput, UpdateDatasetRequest,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use serde_json::Value;
use sqlx::PgPool;



// GET /api/datasets
pub async fn list(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> Result<Json<Vec<Dataset>>, StatusCode> {
    println!("📋 Listing datasets for user {}", user.user_id);

    let datasets = sqlx::query_as::<_, Dataset>(
        r#"
        SELECT id, name, question_count, avg_score, evaluations,
               NULL::text as last_used, created_at::text as created_at
        FROM datasets
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(&user.user_id)
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
    user: AuthUser,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    // Backward-compatible:
    // 1) {name, question_count}
    // 2) {name, description?, questions:[...]}
    if payload.get("questions").is_some() {
        let upload_payload: DatasetUploadRequest =
            serde_json::from_value(payload).map_err(|_| StatusCode::BAD_REQUEST)?;
        let created = create_dataset_with_questions(&pool, upload_payload, &user.user_id).await?;
        return Ok((StatusCode::CREATED, Json(json!(created))));
    }

    let payload: CreateDatasetRequest =
        serde_json::from_value(payload).map_err(|_| StatusCode::BAD_REQUEST)?;
    println!("➕ Creating dataset: {}", payload.name);

    let id = format!("ds_{}", chrono::Utc::now().timestamp());
    let dataset = sqlx::query_as::<_, Dataset>(
        r#"
        INSERT INTO datasets (id, name, question_count, avg_score, evaluations, created_at, user_id)
        VALUES ($1, $2, $3, NULL, 0, NOW(), $4)
        RETURNING id, name, question_count, avg_score, evaluations,
                  NULL::text as last_used, created_at::text as created_at
        "#,
    )
    .bind(id)
    .bind(payload.name)
    .bind(payload.question_count)
    .bind(&user.user_id)
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
    user: AuthUser,
    Json(payload): Json<DatasetUploadRequest>,
) -> Result<(StatusCode, Json<DatasetWithQuestions>), StatusCode> {
    let created = create_dataset_with_questions(&pool, payload, &user.user_id).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

// GET /api/datasets/:id
pub async fn get(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Dataset>, StatusCode> {
    println!("🔍 Getting dataset: {}", id);

    let dataset = sqlx::query_as::<_, Dataset>(
        r#"
        SELECT id, name, question_count, avg_score, evaluations,
               NULL::text as last_used, created_at::text as created_at
        FROM datasets WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(&user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(dataset))
}

// GET /api/datasets/:id/questions
pub async fn get_questions(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(dataset_id): Path<String>,
) -> Result<Json<Vec<Question>>, StatusCode> {
    println!("📋 Getting questions for dataset: {}", dataset_id);

    // Gate on dataset ownership; questions inherit their dataset's owner.
    let owns: Option<String> =
        sqlx::query_scalar("SELECT id FROM datasets WHERE id = $1 AND user_id = $2")
            .bind(&dataset_id)
            .bind(&user.user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if owns.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let questions = sqlx::query_as::<_, Question>(
        r#"
        SELECT id, dataset_id, question_text, expected_answer,
               COALESCE(question_order, 0) as question_order,
               variable_values, tags, difficulty, case_type
        FROM questions
        WHERE dataset_id = $1
        ORDER BY question_order
        "#,
    )
    .bind(&dataset_id)
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
    user: AuthUser,
    Path(dataset_id): Path<String>,
    Json(payload): Json<QuestionInput>,
) -> Result<(StatusCode, Json<Question>), StatusCode> {
    println!("➕ Adding question to dataset: {}", dataset_id);

    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM datasets WHERE id = $1 AND user_id = $2")
            .bind(&dataset_id)
            .bind(&user.user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(question_order), -1) + 1 FROM questions WHERE dataset_id = $1",
    )
    .bind(&dataset_id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let created = sqlx::query_as::<_, Question>(
        r#"
        INSERT INTO questions
            (dataset_id, question_text, expected_answer, question_order,
             variable_values, tags, difficulty, case_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, dataset_id, question_text, expected_answer, question_order,
                  variable_values, tags, difficulty, case_type
        "#,
    )
    .bind(&dataset_id)
    .bind(&payload.question)
    .bind(&payload.answer)
    .bind(next_order)
    .bind(&payload.variable_values)
    .bind(&payload.tags)
    .bind(&payload.difficulty)
    .bind(&payload.case_type)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE datasets SET question_count = question_count + 1 WHERE id = $1")
        .bind(&dataset_id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(created)))
}

// PUT /api/datasets/:id
pub async fn update(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDatasetRequest>,
) -> Result<Json<Dataset>, StatusCode> {
    println!("✏️  Updating dataset: {}", id);

    let dataset = sqlx::query_as::<_, Dataset>(
        r#"
        UPDATE datasets
        SET name = COALESCE($2, name)
        WHERE id = $1 AND user_id = $3
        RETURNING id, name, question_count, avg_score, evaluations,
                  NULL::text as last_used, created_at::text as created_at
        "#,
    )
    .bind(&id)
    .bind(&payload.name)
    .bind(&user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(dataset))
}

// DELETE /api/datasets/:id
pub async fn delete(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    println!("🗑️  Deleting dataset: {}", id);

    // Verify ownership BEFORE the cascade — otherwise a non-owner's request would
    // delete the dataset's eval rows before the final (scoped) delete fails.
    let owns: Option<String> =
        sqlx::query_scalar("SELECT id FROM datasets WHERE id = $1 AND user_id = $2")
            .bind(&id)
            .bind(&user.user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if owns.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut tx = pool.begin().await.map_err(|e| {
        eprintln!("Database error (begin tx) deleting dataset {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    sqlx::query(
        r#"
        DELETE FROM evaluation_details
        WHERE question_id IN (SELECT id FROM questions WHERE dataset_id = $1)
           OR run_id      IN (SELECT id FROM evaluation_runs WHERE dataset_id = $1)
        "#,
    )
    .bind(&id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        eprintln!(
            "Database error deleting evaluation_details for {}: {}",
            id, e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    sqlx::query("DELETE FROM evaluation_runs WHERE dataset_id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            eprintln!("Database error deleting evaluation_runs for {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let result = sqlx::query("DELETE FROM datasets WHERE id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            eprintln!("Database error deleting dataset {}: {}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    tx.commit().await.map_err(|e| {
        eprintln!("Database error (commit) deleting dataset {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(DeleteResponse { deleted: true, id }))
}

async fn create_dataset_with_questions(
    pool: &PgPool,
    payload: DatasetUploadRequest,
    user_id: &str,
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
        INSERT INTO datasets (id, name, description, question_count, avg_score, evaluations, created_at, user_id)
        VALUES ($1, $2, $3, $4, NULL, 0, NOW(), $5)
        RETURNING id, name, question_count, avg_score, evaluations,
                  NULL::text as last_used, created_at::text as created_at
        "#,
    )
    .bind(&dataset_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(question_count)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        eprintln!("Database error creating dataset: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut created_questions: Vec<Question> = Vec::new();

    for (idx, q) in payload.questions.iter().enumerate() {
        let question = sqlx::query_as::<_, Question>(
            r#"
            INSERT INTO questions
                (dataset_id, question_text, expected_answer, question_order,
                 variable_values, tags, difficulty, case_type)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, dataset_id, question_text, expected_answer, question_order,
                      variable_values, tags, difficulty, case_type
            "#,
        )
        .bind(&dataset.id)
        .bind(&q.question)
        .bind(&q.answer)
        .bind(idx as i32)
        .bind(&q.variable_values)
        .bind(&q.tags)
        .bind(&q.difficulty)
        .bind(&q.case_type)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            eprintln!("Database error inserting question {}: {}", idx, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        created_questions.push(question);
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    println!(
        "✅ Uploaded {} questions to dataset {}",
        question_count, dataset_id
    );

    Ok(DatasetWithQuestions {
        dataset,
        questions: created_questions,
    })
}
