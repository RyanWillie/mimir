//! Prompt templates for different memory processing tasks

use serde::{Deserialize, Serialize};

/// Different types of tasks the LLM can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptType {
    /// Extract memorable content from text
    Extract,
    /// Summarize memory content
    Summarize,
    /// Resolve conflicts between memories
    Resolve,
    /// Classify memory content
    Classify,
}

/// Prompt template manager
pub struct PromptManager {
    extract_template: String,
    summarize_template: String,
    resolve_template: String,
    classify_template: String,
}

impl PromptManager {
    /// Create a new prompt manager with default templates
    pub fn new() -> Self {
        Self {
            extract_template: Self::default_extract_template(),
            summarize_template: Self::default_summarize_template(),
            resolve_template: Self::default_resolve_template(),
            classify_template: Self::default_classify_template(),
        }
    }
    
    /// Build a prompt for memory extraction
    pub fn build_extract_prompt(&self, text: &str) -> String {
        self.extract_template.replace("{INPUT}", text)
    }
    
    /// Build a prompt for memory summarization
    pub fn build_summarize_prompt(&self, content: &str, max_tokens: usize) -> String {
        self.summarize_template
            .replace("{CONTENT}", content)
            .replace("{MAX_TOKENS}", &max_tokens.to_string())
    }
    
    /// Build a prompt for conflict resolution
    pub fn build_resolve_prompt(&self, existing: &str, new: &str, similarity: f32) -> String {
        self.resolve_template
            .replace("{EXISTING}", existing)
            .replace("{NEW}", new)
            .replace("{SIMILARITY}", &format!("{:.2}", similarity))
    }
    
    /// Build a prompt for memory classification
    pub fn build_classify_prompt(&self, content: &str) -> String {
        self.classify_template.replace("{CONTENT}", content)
    }
    
    /// Default template for memory extraction
    fn default_extract_template() -> String {
        r#"<bos><start_of_turn>user
You are a memory extraction assistant. Your task is to identify and extract memorable, important information from the given text.

Extract information that is:
- Actionable (tasks, appointments, reminders)
- Factual (names, dates, locations, decisions)
- Personal (preferences, experiences, learnings)
- Contextual (project details, relationships, goals)

For each piece of memorable information, provide:
1. The extracted content (concise but complete)
2. A confidence score (0.0-1.0)
3. A suggested category (personal, work, health, financial, other)

Input text:
{INPUT}

Respond in JSON format:
{
  "memories": [
    {
      "content": "extracted memory content",
      "confidence": 0.85,
      "category": "work",
      "context": "brief context if needed"
    }
  ]
}
<end_of_turn>
<start_of_turn>model
"#.to_string()
    }
    
    /// Default template for memory summarization
    fn default_summarize_template() -> String {
        r#"<bos><start_of_turn>user
You are a memory summarization assistant. Condense the given memory content while preserving all important information.

Requirements:
- Keep all key facts, dates, names, and actionable items
- Maintain clarity and context
- Target approximately {MAX_TOKENS} tokens or less
- Preserve the original meaning and intent

Memory to summarize:
{CONTENT}

Provide a concise summary that captures the essential information:
<end_of_turn>
<start_of_turn>model
"#.to_string()
    }
    
    /// Default template for conflict resolution
    fn default_resolve_template() -> String {
        r#"<bos><start_of_turn>user
You are a memory conflict resolution assistant. Two similar memories have been detected (similarity: {SIMILARITY}). Determine the best action to take.

Existing memory:
{EXISTING}

New memory:
{NEW}

Analyze these memories and choose the best action:
- MERGE: Combine information from both memories
- REPLACE: New memory supersedes the existing one
- KEEP_BOTH: Memories are different enough to keep separately
- DISCARD: New memory adds no value

Respond in JSON format:
{
  "action": "MERGE|REPLACE|KEEP_BOTH|DISCARD",
  "reason": "brief explanation of your decision",
  "result": "final memory content (if merging or replacing)"
}
<end_of_turn>
<start_of_turn>model
"#.to_string()
    }
    
    /// Default template for memory classification
    fn default_classify_template() -> String {
        r#"<bos><start_of_turn>user
You are a memory classification assistant. Classify the given memory content into the most appropriate category.

Categories:
- personal: Personal life, relationships, hobbies, preferences
- work: Professional tasks, meetings, projects, career
- health: Medical information, fitness, wellness, appointments
- financial: Money, investments, bills, purchases, budgets
- other: Anything that doesn't fit the above categories

Memory content:
{CONTENT}

Respond with just the category name (lowercase):
<end_of_turn>
<start_of_turn>model
"#.to_string()
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse extraction response from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResponse {
    pub memories: Vec<ExtractedMemory>,
}

/// A memory extracted by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub content: String,
    pub confidence: f32,
    pub category: String,
    pub context: Option<String>,
}

/// Parse conflict resolution response from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionResponse {
    pub action: String,
    pub reason: String,
    pub result: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prompt_building() {
        let manager = PromptManager::new();
        
        let extract_prompt = manager.build_extract_prompt("I need to call John tomorrow at 3pm");
        assert!(extract_prompt.contains("I need to call John tomorrow at 3pm"));
        assert!(extract_prompt.contains("JSON format"));
        
        let summarize_prompt = manager.build_summarize_prompt("Long content here", 150);
        assert!(summarize_prompt.contains("Long content here"));
        assert!(summarize_prompt.contains("150"));
    }
    
    #[test]
    fn test_json_parsing() {
        let json = r#"{
            "memories": [
                {
                    "content": "Call John tomorrow at 3pm",
                    "confidence": 0.9,
                    "category": "personal",
                    "context": "scheduled call"
                }
            ]
        }"#;
        
        let response: ExtractionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.memories.len(), 1);
        assert_eq!(response.memories[0].content, "Call John tomorrow at 3pm");
        assert_eq!(response.memories[0].confidence, 0.9);
    }
} 