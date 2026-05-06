// src/utils/template_parser.rs

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
fn test_extract_variables() {
    let template = "You are a {{ROLE}}. Answer {{QUESTION}}";
    let vars = extract_variables(template);
    assert_eq!(vars, vec!["ROLE", "QUESTION"]);
}

#[test]
fn test_fill_template() {
    let template = "You are a {{ROLE}}. Answer {{QUESTION}}";
    let values = json!({"ROLE": "teacher", "QUESTION": "What is 2+2?"});
    let filled = fill_template(template, &values);
    assert_eq!(filled, "You are a teacher. Answer What is 2+2?");
}
