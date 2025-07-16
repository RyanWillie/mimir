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
    search_summary_template: String,
}

impl PromptManager {
    /// Create a new prompt manager with default templates
    pub fn new() -> Self {
        Self {
            extract_template: Self::default_extract_template(),
            summarize_template: Self::default_summarize_template(),
            resolve_template: Self::default_resolve_template(),
            classify_template: Self::default_classify_template(),
            search_summary_template: Self::default_search_summary_template(),
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
    
    /// Build a prompt for search result summarization
    pub fn build_search_summary_prompt(&self, query: &str, results: &[String]) -> String {
        let results_text = results.join("\n\n");
        self.search_summary_template
            .replace("{QUERY}", query)
            .replace("{RESULTS}", &results_text)
    }
    
    /// Default template for memory extraction
    fn default_extract_template() -> String {
        r#"You are an expert memory extraction system. Your task is to identify and extract "user memories" from the provided user message.

A "user memory" is a concise, actionable piece of information about the user's current goal, preference, state, or an ongoing topic that the agent should remember for future interactions.

Extract only genuinely relevant and actionable memories. Exclude conversational filler, acknowledgments, or trivial statements.

Output the extracted memories as a JSON array of strings. Each string should be a brief, clear summary of the memory. If no relevant memories are found, output an empty JSON array `[]`.

---
**Examples:**

**User Message:** "Thanks for that, I really appreciate it."
**Output:** `[]`

**User Message:** "I am wanting to find a good book to read, what do you recommend?"
**Output:** `["User is wanting to find a good book to read"]`

**User Message:** "My favorite color is blue, and I live in London."
**Output:** `["User's favorite color is blue", "User lives in London"]`

**User Message:** "Can you help me plan a trip to Paris next summer? I'm interested in art museums and prefer quiet places."
**Output:** `["User wants to plan a trip to Paris next summer", "User is interested in art museums", "User prefers quiet places"]`

**User Message:** "Okay, sounds good."
**Output:** `[]`

---
**Your Turn:**

Input text:
{INPUT}"#.to_string()
    }
    
    /// Default template for memory summarization
    fn default_summarize_template() -> String {
        r#"You are an expert memory summarization assistant. Your task is to condense provided content into a concise summary. The goal is to extract and retain only the most critical and relevant information to reduce token usage while preserving utility for future interactions.

**Key Requirements for Summarization:**

* **Extract Core Facts & Concepts:** Identify and preserve all essential facts, key concepts, significant decisions, and concrete details.
* **Identify Actionable Information & Implications:** Capture any explicit or implicit requests, goals, preferences, tasks, strategic considerations, potential implications, or information that requires future action, attention, or influences agent behavior/decisions.
* **Maintain Context and Intent:** Ensure the summarized memory accurately reflects the original meaning, purpose, and context of the input. Do not introduce new information or interpretations.
* **Prioritize Relevance:** Focus on information that is most likely to be relevant or impactful for the agent's future operation, decision-making, or interaction with users.
* **Conciseness Target:** Aim for a summary that is approximately 50 tokens or less. Be as succinct as possible without sacrificing critical information.

**Example 1**

* **Input Content:** "Considering making episodic memory reflection a cloud-based feature rather than local processing - recognizing the computational overhead and complexity of running continuous LLM reflections locally."
* **Desired Output Summary:** "Decision point: Episodic memory reflection likely better as cloud-based feature due to local LLM computational overhead."

**Example 2**

* **Input Content:** "I'm having trouble getting gemma3 1B to work with candle core, should this be straight forward?"
* **Desired Output Summary:** "User is having trouble getting gemma3 1B to work with candle core"



Memory to summarize:
{CONTENT}

Summary:"#.to_string()
    }
    
    /// Default template for conflict resolution
    fn default_resolve_template() -> String {
        r#"You are a memory conflict resolution assistant. Two similar memories have been detected (similarity: {SIMILARITY}). Determine the best action to take.

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

JSON Response:"#.to_string()
    }
    
    /// Default template for memory classification
    fn default_classify_template() -> String {
        r#"You are a memory classification assistant. Classify the given memory content into the most appropriate category.

Categories:
- personal: Personal life, relationships, hobbies, preferences
- work: Professional tasks, meetings, projects, career
- health: Medical information, fitness, wellness, appointments
- financial: Money, investments, bills, purchases, budgets
- other: Anything that doesn't fit the above categories

Memory content:
{CONTENT}

Category:"#.to_string()
    }
    
    /// Default template for search result summarization
    fn default_search_summary_template() -> String {
        r#"
You are a highly focused Information Extractor. Your task is to process a set of SEARCH RESULTS and pull out only the information that directly and clearly answers the user's QUERY.

Your process must follow these strict steps:

    Analyze the Query: Understand the core subject and specific information requested in the QUERY. Identify key terms, concepts, and the type of information sought (e.g., definitions, features, steps, comparisons, examples, causes, effects, etc.).

    Scan for Direct Matches: Read through each SEARCH RESULT meticulously. Look for sentences or phrases that contain the key terms or directly address the specific information type identified in Step 1.

    Extract Relevant Sentences/Phrases: Copy only the sentences or distinct phrases that directly answer or provide relevant details for the QUERY. Do not paraphrase or add your own words.

    Consolidate & List: Combine all extracted sentences/phrases into a single, organized list of bullet points. Each bullet point should be a concise piece of extracted information. If a single point contains multiple sub-details, use sub-bullets.

    Eliminate Redundancy: Review your extracted list. If the same piece of information is present in multiple sentences, include it only once.

    Maintain Originality: Ensure the extracted information retains its original wording and factual accuracy from the SEARCH RESULTS.

    Handle Irrelevance: If, after thorough review, you find no information in the SEARCH RESULTS that directly answers or is highly relevant to the QUERY, your sole output must be: "No relevant information found."

    Exclusive Output: Your output must consist only of the bulleted list of extracted information or the "No relevant information found" statement. Do not include any introductory phrases, conversational elements, or extraneous text.

QUERY:
{QUERY}

SEARCH RESULTS:
{RESULTS}
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
    pub relevance: f32,
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
    fn test_json_parsing() {
        let json = r#"{
            "memories": [
                {
                    "content": "Call John tomorrow at 3pm",
                    "relevance": 0.9,
                    "category": "personal",
                    "context": "scheduled call"
                }
            ]
        }"#;
        
        let response: ExtractionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.memories.len(), 1);
        assert_eq!(response.memories[0].content, "Call John tomorrow at 3pm");
        assert_eq!(response.memories[0].relevance, 0.9);
    }
} 