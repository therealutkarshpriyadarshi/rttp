//! Prompt template engine
//!
//! Provides a template engine for LLM prompts with variable interpolation,
//! conditionals, and loops (Jinja-like syntax).

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Errors that can occur during template processing
#[derive(Debug)]
pub enum TemplateError {
    /// Variable not found in context
    VariableNotFound(String),
    /// Invalid template syntax
    SyntaxError(String),
    /// Error rendering template
    RenderError(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::VariableNotFound(var) => write!(f, "Variable not found: {}", var),
            TemplateError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            TemplateError::RenderError(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for TemplateError {}

/// Result type for template operations
pub type Result<T> = std::result::Result<T, TemplateError>;

/// Context for template rendering
pub type Context = HashMap<String, Value>;

/// Prompt template with variable interpolation
pub struct PromptTemplate {
    /// Raw template string
    template: String,
    /// Variable names used in template
    variables: Vec<String>,
}

impl PromptTemplate {
    /// Parse a template string
    ///
    /// Supports:
    /// - Variable interpolation: `{{variable}}`
    /// - Conditionals: `{% if condition %}...{% endif %}`
    /// - Loops: `{% for item in items %}...{% endfor %}`
    pub fn parse(template: impl Into<String>) -> Result<Self> {
        let template = template.into();
        let variables = Self::extract_variables(&template)?;

        Ok(Self {
            template,
            variables,
        })
    }

    /// Extract variable names from template
    fn extract_variables(template: &str) -> Result<Vec<String>> {
        let var_regex = Regex::new(r"\{\{(\s*\w+\s*)\}\}")
            .map_err(|e| TemplateError::SyntaxError(e.to_string()))?;

        let mut variables = Vec::new();
        for cap in var_regex.captures_iter(template) {
            let var_name = cap[1].trim().to_string();
            if !variables.contains(&var_name) {
                variables.push(var_name);
            }
        }

        Ok(variables)
    }

    /// Get the list of variables used in this template
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// Render the template with the given context
    pub fn render(&self, context: &Context) -> Result<String> {
        let mut result = self.template.clone();

        // Process conditionals
        result = self.process_conditionals(&result, context)?;

        // Process loops
        result = self.process_loops(&result, context)?;

        // Process variable interpolation
        result = self.process_variables(&result, context)?;

        Ok(result)
    }

    /// Process conditional blocks
    fn process_conditionals(&self, text: &str, context: &Context) -> Result<String> {
        let if_regex = Regex::new(r"\{%\s*if\s+(\w+)\s*%\}(.*?)\{%\s*endif\s*%\}")
            .map_err(|e| TemplateError::SyntaxError(e.to_string()))?;

        let mut result = text.to_string();

        for cap in if_regex.captures_iter(text) {
            let var_name = &cap[1];
            let content = &cap[2];
            let full_match = &cap[0];

            let should_include = context
                .get(var_name)
                .map(|v| match v {
                    Value::Bool(b) => *b,
                    Value::Null => false,
                    Value::String(s) => !s.is_empty(),
                    Value::Array(a) => !a.is_empty(),
                    Value::Number(_) => true,
                    Value::Object(_) => true,
                })
                .unwrap_or(false);

            let replacement = if should_include { content } else { "" };
            result = result.replace(full_match, replacement);
        }

        Ok(result)
    }

    /// Process loop blocks
    fn process_loops(&self, text: &str, context: &Context) -> Result<String> {
        let for_regex = Regex::new(r"\{%\s*for\s+(\w+)\s+in\s+(\w+)\s*%\}(.*?)\{%\s*endfor\s*%\}")
            .map_err(|e| TemplateError::SyntaxError(e.to_string()))?;

        let mut result = text.to_string();

        for cap in for_regex.captures_iter(text) {
            let item_name = &cap[1];
            let array_name = &cap[2];
            let template_block = &cap[3];
            let full_match = &cap[0];

            let items = context
                .get(array_name)
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    TemplateError::VariableNotFound(format!("{} (or not an array)", array_name))
                })?;

            let mut rendered_items = String::new();
            for item in items {
                let mut item_context = context.clone();
                item_context.insert(item_name.to_string(), item.clone());

                let rendered = self.process_variables(template_block, &item_context)?;
                rendered_items.push_str(&rendered);
            }

            result = result.replace(full_match, &rendered_items);
        }

        Ok(result)
    }

    /// Process variable interpolation
    fn process_variables(&self, text: &str, context: &Context) -> Result<String> {
        let var_regex = Regex::new(r"\{\{(\s*\w+\s*)\}\}")
            .map_err(|e| TemplateError::SyntaxError(e.to_string()))?;

        let mut result = text.to_string();

        for cap in var_regex.captures_iter(text) {
            let var_name = cap[1].trim();
            let full_match = &cap[0];

            let value = context
                .get(var_name)
                .ok_or_else(|| TemplateError::VariableNotFound(var_name.to_string()))?;

            let replacement = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => String::new(),
                _ => serde_json::to_string(value)
                    .map_err(|e| TemplateError::RenderError(e.to_string()))?,
            };

            result = result.replace(full_match, &replacement);
        }

        Ok(result)
    }
}

/// Builder for creating prompt templates with common patterns
pub struct PromptBuilder {
    system_prompt: Option<String>,
    user_prompt: String,
    examples: Vec<(String, String)>,
}

impl PromptBuilder {
    /// Create a new prompt builder
    pub fn new() -> Self {
        Self {
            system_prompt: None,
            user_prompt: String::new(),
            examples: Vec::new(),
        }
    }

    /// Set the system prompt
    pub fn system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the user prompt
    pub fn user(mut self, prompt: impl Into<String>) -> Self {
        self.user_prompt = prompt.into();
        self
    }

    /// Add an example (input -> output)
    pub fn example(mut self, input: impl Into<String>, output: impl Into<String>) -> Self {
        self.examples.push((input.into(), output.into()));
        self
    }

    /// Build the final prompt
    pub fn build(self) -> String {
        let mut prompt = String::new();

        if let Some(system) = self.system_prompt {
            prompt.push_str(&system);
            prompt.push_str("\n\n");
        }

        if !self.examples.is_empty() {
            prompt.push_str("Examples:\n");
            for (input, output) in &self.examples {
                prompt.push_str(&format!("Input: {}\nOutput: {}\n\n", input, output));
            }
        }

        prompt.push_str(&self.user_prompt);

        prompt
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_variable_interpolation() {
        let template = PromptTemplate::parse("Hello {{name}}!").unwrap();
        let mut context = Context::new();
        context.insert("name".to_string(), json!("World"));

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_multiple_variables() {
        let template =
            PromptTemplate::parse("{{greeting}} {{name}}! You are {{age}} years old.").unwrap();
        let mut context = Context::new();
        context.insert("greeting".to_string(), json!("Hello"));
        context.insert("name".to_string(), json!("Alice"));
        context.insert("age".to_string(), json!(30));

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello Alice! You are 30 years old.");
    }

    #[test]
    fn test_conditional_true() {
        let template =
            PromptTemplate::parse("Hello{% if premium %} Premium User{% endif %}!").unwrap();
        let mut context = Context::new();
        context.insert("premium".to_string(), json!(true));

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello Premium User!");
    }

    #[test]
    fn test_conditional_false() {
        let template =
            PromptTemplate::parse("Hello{% if premium %} Premium User{% endif %}!").unwrap();
        let mut context = Context::new();
        context.insert("premium".to_string(), json!(false));

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello!");
    }

    #[test]
    fn test_loop() {
        let template =
            PromptTemplate::parse("Items: {% for item in items %}{{item}}, {% endfor %}").unwrap();
        let mut context = Context::new();
        context.insert("items".to_string(), json!(["apple", "banana", "cherry"]));

        let result = template.render(&context).unwrap();
        assert_eq!(result, "Items: apple, banana, cherry, ");
    }

    #[test]
    fn test_variable_extraction() {
        let template = PromptTemplate::parse("{{name}} is {{age}} years old").unwrap();
        let vars = template.variables();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"age".to_string()));
    }

    #[test]
    fn test_missing_variable_error() {
        let template = PromptTemplate::parse("Hello {{name}}!").unwrap();
        let context = Context::new();

        let result = template.render(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_prompt_builder() {
        let prompt = PromptBuilder::new()
            .system("You are a helpful assistant")
            .example("Hello", "Hi there!")
            .user("What is Rust?")
            .build();

        assert!(prompt.contains("You are a helpful assistant"));
        assert!(prompt.contains("Examples:"));
        assert!(prompt.contains("Input: Hello"));
        assert!(prompt.contains("Output: Hi there!"));
        assert!(prompt.contains("What is Rust?"));
    }
}
