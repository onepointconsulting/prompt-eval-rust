use sqlx::PgPool;
use std::fs;
use std::path::Path;

pub async fn maybe_run(pool: &PgPool) -> Result<(), sqlx::Error> {
    let path = match std::env::var("BOOTSTRAP_SCRIPT") {
        Ok(p) if !p.is_empty() => p,
        _ => return Ok(()),
    };

    if !Path::new(&path).exists() {
        return Err(sqlx::Error::Configuration(
            format!("BOOTSTRAP_SCRIPT file not found: {path}").into(),
        ));
    }

    println!("📜 Running bootstrap script: {path}");

    let sql = fs::read_to_string(&path).map_err(|e| {
        sqlx::Error::Configuration(format!("Failed to read BOOTSTRAP_SCRIPT ({path}): {e}").into())
    })?;

    for statement in split_statements(&sql) {
        sqlx::raw_sql(&statement).execute(pool).await?;
    }

    println!("✅ Bootstrap script complete");

    Ok(())
}

fn strip_line_comments(sql: &str) -> String {
    sql.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("--") {
                ""
            } else if let Some(idx) = line.find("--") {
                &line[..idx]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn split_statements(sql: &str) -> Vec<String> {
    strip_line_comments(sql)
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}
