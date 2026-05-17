pub fn extract_variables(template: &str) -> Vec<String> {
    let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
    let mut vars: Vec<String> = re
        .captures_iter(template)
        .map(|cap| cap[1].to_string())
        .collect();

    // Deduplicate while preserving order
    vars.sort();
    vars.dedup();

    vars
}

pub fn fill_template(template: &str, values: &serde_json::Value) -> String {
    let mut result = template.to_string();

    if let Some(obj) = values.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{{{}}}}}", key);
            if let Some(val_str) = value.as_str() {
                result = result.replace(&placeholder, val_str);
            }
        }
    }

    result
}

#[cfg(test)]
use serde_json::json;

#[test]
// results are sorted alphabetically — QUESTION before ROLE
fn test_extract_variables() {
    let template = "You are a {{ROLE}}. Answer {{QUESTION}}";
    let vars = extract_variables(template);
    assert_eq!(vars, vec!["QUESTION", "ROLE"]);
}

#[test]
// single braces {VAR} must not be extracted — only {{VAR}} is valid syntax
fn extract_variables_double_braces() {
    let template = "You are a {ROLE}. Answer {QUESTION}";
    let vars = extract_variables(template);
    assert!(vars.is_empty());
}

#[test]
// all placeholders replaced when every key is present in the values map
fn test_fill_template() {
    let template = "You are a {{ROLE}}. Answer {{QUESTION}}";
    let values = json!({"ROLE": "teacher", "QUESTION": "What is 2+2?"});
    let filled = fill_template(template, &values);
    assert_eq!(filled, "You are a teacher. Answer What is 2+2?");
}

#[test]
// plain text with no placeholders returns an empty variable list
fn extract_variables_no_vars() {
    let template = "You are a helpful assistant.";
    let vars = extract_variables(template);
    assert!(vars.is_empty());
}

#[test]
// multiple distinct variables are all captured and returned sorted
fn extract_variables_multiple() {
    let template = "You are a helpful assistant. Answer {{QUESTION}} and {{QUESTION2}}";
    let vars = extract_variables(template);
    assert_eq!(vars, vec!["QUESTION", "QUESTION2"]);
}

#[test]
// every placeholder replaced when all keys are provided
fn fill_template_all_present() {
    let template = "You are a helpful assistant. Answer {{QUESTION}} and {{QUESTION2}}";
    let values = json!({"QUESTION": "What is 2+2?", "QUESTION2": "What is 3+3?"});
    let filled = fill_template(template, &values);
    assert_eq!(
        filled,
        "You are a helpful assistant. Answer What is 2+2? and What is 3+3?"
    );
}

#[test]
// missing key leaves the placeholder intact — prevents silently broken prompts
fn fill_template_missing_var() {
    let template = "You are a helpful assistant. Answer {{QUESTION}} and {{QUESTION2}}";
    let values = json!({"QUESTION": "What is 2+2?"});
    let filled = fill_template(template, &values);
    assert_eq!(
        filled,
        "You are a helpful assistant. Answer What is 2+2? and {{QUESTION2}}"
    );
}

#[test]
// extra keys in the values map are silently ignored — only matched placeholders are replaced
fn fill_template_extra_key() {
    let template = "You are a helpful assistant. Answer {{QUESTION}} and {{QUESTION2}}";
    let values = json!({"QUESTION": "What is 2+2?", "QUESTION2": "What is 3+3?", "EXTRA": "extra"});
    let filled = fill_template(template, &values);
    assert_eq!(
        filled,
        "You are a helpful assistant. Answer What is 2+2? and What is 3+3?"
    );
}
