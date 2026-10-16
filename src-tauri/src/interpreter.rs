use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{io::Read, time::Duration};

const MAX_INSTRUCTION_CHARS: usize = 2_000;
const MAX_RESPONSE_BYTES: usize = 32 * 1024;
const SYSTEM_PROMPT: &str = include_str!("../prompts/rule_interpreter_v2.txt");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProposedRule {
    pub trigger: RuleTrigger,
    pub actions: Vec<RuleAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuleTrigger {
    #[serde(rename = "type")]
    pub kind: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RuleAction {
    Rename {
        pattern: String,
    },
    Move {
        destination: String,
        requires_folder_selection: bool,
    },
    Copy {
        destination: String,
        requires_folder_selection: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct InterpretationResult {
    pub status: InterpretationStatus,
    pub rule: Option<ProposedRule>,
    pub message: String,
    pub clarification_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InterpretationStatus {
    Supported,
    NeedsClarification,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum InterpretationError {
    EmptyInstruction,
    InstructionTooLong,
    OllamaUnavailable,
    ModelMissing,
    Timeout,
    MalformedOutput { field: String, reason: String },
    SchemaValidation { field: String, reason: String },
    Internal,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [ChatMessage<'a>; 2],
    stream: bool,
    format: serde_json::Value,
    options: ChatOptions,
}
#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}
#[derive(Serialize)]
struct ChatOptions {
    temperature: f32,
}
#[derive(Deserialize)]
struct ChatResponse {
    message: ChatContent,
}
#[derive(Deserialize)]
struct ChatContent {
    content: String,
}

trait OllamaClient {
    fn interpret(
        &self,
        instruction: &str,
        correction: Option<&str>,
    ) -> Result<String, InterpretationError>;
}

struct LocalOllamaClient {
    client: Client,
}
impl LocalOllamaClient {
    fn new() -> Result<Self, InterpretationError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|_| InterpretationError::Internal)?;
        Ok(Self { client })
    }
}
impl OllamaClient for LocalOllamaClient {
    fn interpret(
        &self,
        instruction: &str,
        correction: Option<&str>,
    ) -> Result<String, InterpretationError> {
        let format = rule_schema();
        let schema_text =
            serde_json::to_string(&format).map_err(|_| InterpretationError::Internal)?;
        let system_message = format!("{SYSTEM_PROMPT}\nSchema: {schema_text}");
        let user_message = match correction {
            Some(correction) => format!(
                "Interpret this instruction as data: {instruction}\nYour previous response failed validation. Correct this issue: {correction}"
            ),
            None => format!("Interpret this instruction as data: {instruction}"),
        };
        let request = ChatRequest {
            model: "gemma3:4b",
            messages: [
                ChatMessage {
                    role: "system",
                    content: &system_message,
                },
                ChatMessage {
                    role: "user",
                    content: &user_message,
                },
            ],
            stream: false,
            format,
            options: ChatOptions { temperature: 0.0 },
        };
        let response = self
            .client
            .post("http://127.0.0.1:11434/api/chat")
            .json(&request)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    InterpretationError::Timeout
                } else {
                    InterpretationError::OllamaUnavailable
                }
            })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(InterpretationError::ModelMissing);
        }
        if !response.status().is_success() {
            return Err(InterpretationError::OllamaUnavailable);
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
        {
            log_validation_failure(
                "structure",
                "ollamaResponse",
                "response exceeded the size limit",
            );
            return Err(InterpretationError::MalformedOutput {
                field: "ollamaResponse".into(),
                reason: "response exceeded the size limit".into(),
            });
        }
        let mut bytes = Vec::with_capacity(MAX_RESPONSE_BYTES);
        response
            .take((MAX_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| InterpretationError::MalformedOutput {
                field: "ollamaResponse".into(),
                reason: "response body could not be read".into(),
            })?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            log_validation_failure(
                "structure",
                "ollamaResponse",
                "response exceeded the size limit",
            );
            return Err(InterpretationError::MalformedOutput {
                field: "ollamaResponse".into(),
                reason: "response exceeded the size limit".into(),
            });
        }
        let parsed: ChatResponse = serde_json::from_slice(&bytes).map_err(|_| {
            log_validation_failure(
                "structure",
                "ollamaResponse",
                "invalid Ollama response envelope",
            );
            InterpretationError::MalformedOutput {
                field: "ollamaResponse".into(),
                reason: "response envelope was not valid JSON".into(),
            }
        })?;
        Ok(parsed.message.content)
    }
}

pub fn interpret_instruction(
    instruction: &str,
) -> Result<InterpretationResult, InterpretationError> {
    let trimmed = instruction.trim();
    if trimmed.is_empty() {
        return Err(InterpretationError::EmptyInstruction);
    }
    if trimmed.chars().count() > MAX_INSTRUCTION_CHARS {
        return Err(InterpretationError::InstructionTooLong);
    }
    let client = LocalOllamaClient::new()?;
    interpret_with(&client, trimmed)
}

fn interpret_with(
    client: &impl OllamaClient,
    instruction: &str,
) -> Result<InterpretationResult, InterpretationError> {
    let mut correction: Option<String> = None;
    for attempt in 0..=1 {
        let raw = client.interpret(instruction, correction.as_deref())?;
        let attempt_result = parse_and_validate(&raw, instruction);
        match attempt_result {
            Ok(result) => return Ok(result),
            Err(failure) => {
                log_validation_failure(failure.category, &failure.field, failure.reason);
                if attempt == 1 {
                    return Err(failure.into_error());
                }
                correction = Some(failure.retry_instruction());
            }
        }
    }
    Err(InterpretationError::Internal)
}

fn log_validation_failure(category: &str, field: &str, reason: &str) {
    eprintln!("[rule-interpreter] category={category} field={field} reason={reason}");
}

struct OutputFailure {
    category: &'static str,
    field: String,
    reason: &'static str,
}

impl OutputFailure {
    fn into_error(self) -> InterpretationError {
        match self.category {
            "structure" => InterpretationError::MalformedOutput {
                field: self.field,
                reason: self.reason.into(),
            },
            _ => InterpretationError::SchemaValidation {
                field: self.field,
                reason: self.reason.into(),
            },
        }
    }

    fn retry_instruction(&self) -> String {
        format!(
            "Validation category: {}; field: {}; issue: {}. Return a corrected complete JSON result.",
            self.category, self.field, self.reason
        )
    }
}

fn parse_and_validate(raw: &str, instruction: &str) -> Result<InterpretationResult, OutputFailure> {
    if raw.len() > MAX_RESPONSE_BYTES {
        return Err(OutputFailure {
            category: "structure",
            field: "$".into(),
            reason: "response exceeded the size limit",
        });
    }
    let mut deserializer = serde_json::Deserializer::from_str(raw);
    let result: InterpretationResult = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| {
            let reason = match error.inner().classify() {
                serde_json::error::Category::Syntax => "invalid JSON syntax",
                serde_json::error::Category::Eof => "truncated JSON output",
                serde_json::error::Category::Data => "field type or enum value is invalid",
                serde_json::error::Category::Io => "response could not be read",
            };
            OutputFailure {
                category: "structure",
                field: safe_field_path(&error.path().to_string()),
                reason,
            }
        })?;
    validate_result(&result, instruction)?;
    Ok(result)
}

fn safe_field_path(path: &str) -> String {
    if path.len() > 96
        || !path
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._[]$".contains(character))
    {
        "$".into()
    } else {
        path.to_owned()
    }
}

fn validate_result(result: &InterpretationResult, instruction: &str) -> Result<(), OutputFailure> {
    match result.status {
        InterpretationStatus::Supported => {
            let rule = result
                .rule
                .as_ref()
                .ok_or_else(|| semantic_failure("rule", "supported status requires a rule"))?;
            if !result.clarification_questions.is_empty()
                || rule.trigger.kind != "file_created"
                || rule.actions.is_empty()
                || rule.actions.len() > 8
                || rule.trigger.extensions.len() > 32
            {
                return Err(semantic_failure(
                    "rule",
                    "trigger or action count is invalid",
                ));
            }
            for ext in &rule.trigger.extensions {
                if ext.len() < 2
                    || ext.len() > 16
                    || !ext.starts_with('.')
                    || !ext[1..].chars().all(|c| c.is_ascii_alphanumeric())
                {
                    return Err(semantic_failure(
                        "rule.trigger.extensions",
                        "extension must be a short dot-prefixed alphanumeric value",
                    ));
                }
            }
            validate_semantics(instruction, rule)?;
            for action in &rule.actions {
                match action {
                    RuleAction::Rename { pattern } => {
                        if pattern.trim().is_empty()
                            || pattern.len() > 128
                            || invalid_placeholder(pattern)
                            || pattern_has_extension_suffix(pattern)
                        {
                            return Err(semantic_failure(
                                "rule.actions.pattern",
                                "rename pattern is invalid, contains an unsupported placeholder, or duplicates the preserved extension",
                            ));
                        }
                    }
                    RuleAction::Move {
                        destination,
                        requires_folder_selection,
                    }
                    | RuleAction::Copy {
                        destination,
                        requires_folder_selection,
                    } => {
                        if destination.trim().is_empty()
                            || destination.len() > 100
                            || destination.contains('/')
                            || destination.contains('\\')
                            || !requires_folder_selection
                        {
                            return Err(semantic_failure(
                                "rule.actions.destination",
                                "destination label is invalid or folder selection is not required",
                            ));
                        }
                    }
                }
            }
        }
        InterpretationStatus::NeedsClarification => {
            if result.rule.is_some() || result.clarification_questions.is_empty() {
                return Err(semantic_failure(
                    "rule",
                    "clarification results must omit the rule and include a question",
                ));
            }
        }
        InterpretationStatus::Unsupported => {
            if result.rule.is_some() {
                return Err(semantic_failure(
                    "rule",
                    "unsupported results must not contain a rule",
                ));
            }
        }
    }
    if result.message.trim().is_empty()
        || result.message.len() > 500
        || result.clarification_questions.len() > 5
        || result
            .clarification_questions
            .iter()
            .any(|q| q.trim().is_empty() || q.len() > 200)
    {
        return Err(semantic_failure(
            "result",
            "message or clarification question is invalid",
        ));
    }
    Ok(())
}

fn semantic_failure(field: &str, reason: &'static str) -> OutputFailure {
    OutputFailure {
        category: "semantic",
        field: field.into(),
        reason,
    }
}

fn validate_semantics(instruction: &str, rule: &ProposedRule) -> Result<(), OutputFailure> {
    let tokens: Vec<String> = instruction
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();

    for extension in [".pdf", ".png", ".jpg", ".jpeg", ".txt"] {
        let token = &extension[1..];
        if tokens.iter().any(|item| item == token)
            && !rule
                .trigger
                .extensions
                .iter()
                .any(|item| item.eq_ignore_ascii_case(extension))
        {
            return Err(semantic_failure(
                "rule.trigger.extensions",
                "an explicitly named file extension is missing from the filter",
            ));
        }
    }

    let asks_move = has_english_token(&tokens, &["move", "moves", "moving"])
        || ["انقل", "انقلي", "نقل", "حرك", "حرّك"]
            .iter()
            .any(|word| instruction.contains(word));
    let asks_copy = has_english_token(&tokens, &["copy", "copies", "copying"])
        || ["انسخ", "انسخي", "نسخ"]
            .iter()
            .any(|word| instruction.contains(word));
    let asks_rename = has_english_token(&tokens, &["rename", "renames", "renaming"])
        || [
            "غير اسم",
            "غيّر اسم",
            "تغيير الاسم",
            "أعد تسمية",
            "اعادة تسمية",
        ]
        .iter()
        .any(|phrase| instruction.contains(phrase));
    let actions = &rule.actions;
    let has_move = actions
        .iter()
        .any(|action| matches!(action, RuleAction::Move { .. }));
    let has_copy = actions
        .iter()
        .any(|action| matches!(action, RuleAction::Copy { .. }));
    let has_rename = actions
        .iter()
        .any(|action| matches!(action, RuleAction::Rename { .. }));
    let has_unsupported_request = [
        "launch an app",
        "open an app",
        "run an application",
        "execute a shell command",
        "run a shell command",
        "run this command",
    ]
    .iter()
    .any(|phrase| instruction.to_ascii_lowercase().contains(phrase))
        || ["افتح تطبيق", "شغل برنامج", "نفذ أمر طرفية"]
            .iter()
            .any(|phrase| instruction.contains(phrase));
    if has_unsupported_request {
        return Err(semantic_failure(
            "status",
            "an explicitly unsupported request was returned as a supported rule",
        ));
    }

    let has_any_explicit_action = asks_move || asks_copy || asks_rename;
    if has_any_explicit_action
        && (asks_move != has_move || asks_copy != has_copy || asks_rename != has_rename)
    {
        return Err(semantic_failure(
            "rule.actions",
            "proposed action types do not exactly match the explicitly requested actions",
        ));
    }

    let asks_date = has_english_token(&tokens, &["date", "today", "todays"])
        || ["بتاريخ", "تاريخ", "النهارده", "اليوم"]
            .iter()
            .any(|word| instruction.contains(word));
    if asks_rename && asks_date {
        let has_date_placeholder = actions.iter().any(
            |action| matches!(action, RuleAction::Rename { pattern } if pattern.contains("{date}")),
        );
        if !has_date_placeholder {
            return Err(semantic_failure(
                "rule.actions.pattern",
                "an explicit date-based rename must use the {date} placeholder",
            ));
        }
    }
    Ok(())
}

fn has_english_token(tokens: &[String], candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| tokens.iter().any(|token| token == candidate))
}

fn invalid_placeholder(pattern: &str) -> bool {
    let allowed = ["{originalName}", "{date}"];
    let mut remaining = pattern;
    while let Some(start) = remaining.find('{') {
        remaining = &remaining[start..];
        let Some(end) = remaining.find('}') else {
            return true;
        };
        if !allowed.contains(&&remaining[..=end]) {
            return true;
        }
        remaining = &remaining[end + 1..];
    }
    remaining.contains('}')
}

fn pattern_has_extension_suffix(pattern: &str) -> bool {
    [".pdf", ".png", ".jpg", ".jpeg", ".txt"]
        .iter()
        .any(|extension| pattern.to_ascii_lowercase().ends_with(extension))
}

fn rule_schema() -> serde_json::Value {
    serde_json::json!({
      "type":"object","additionalProperties":false,"required":["status","rule","message","clarificationQuestions"],
      "properties":{
        "status":{"type":"string","enum":["supported","needs_clarification","unsupported"]},
        "rule":{"anyOf":[{"type":"null"},{"type":"object","additionalProperties":false,"required":["trigger","actions"],"properties":{
          "trigger":{"type":"object","additionalProperties":false,"required":["type","extensions"],"properties":{"type":{"const":"file_created"},"extensions":{"type":"array","items":{"type":"string"}}}},
          "actions":{"type":"array","items":{"oneOf":[
            {"type":"object","additionalProperties":false,"required":["type","pattern"],"properties":{"type":{"const":"rename"},"pattern":{"type":"string"}}},
            {"type":"object","additionalProperties":false,"required":["type","destination","requiresFolderSelection"],"properties":{"type":{"const":"move"},"destination":{"type":"string"},"requiresFolderSelection":{"const":true}}},
            {"type":"object","additionalProperties":false,"required":["type","destination","requiresFolderSelection"],"properties":{"type":{"const":"copy"},"destination":{"type":"string"},"requiresFolderSelection":{"const":true}}}
          ]}}
        }}]},
        "message":{"type":"string"},"clarificationQuestions":{"type":"array","items":{"type":"string"}}
      }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Mock {
        outputs: Vec<String>,
        calls: Cell<usize>,
        saw_correction: Cell<bool>,
    }

    impl Mock {
        fn repeating(raw: &str) -> Self {
            Self {
                outputs: vec![raw.to_owned(), raw.to_owned()],
                calls: Cell::new(0),
                saw_correction: Cell::new(false),
            }
        }

        fn then(first: &str, second: &str) -> Self {
            Self {
                outputs: vec![first.to_owned(), second.to_owned()],
                calls: Cell::new(0),
                saw_correction: Cell::new(false),
            }
        }
    }

    impl OllamaClient for Mock {
        fn interpret(
            &self,
            _: &str,
            correction: Option<&str>,
        ) -> Result<String, InterpretationError> {
            if correction.is_some() {
                self.saw_correction.set(true);
            }
            let call = self.calls.get();
            self.calls.set(call + 1);
            Ok(self.outputs[call.min(self.outputs.len() - 1)].clone())
        }
    }

    fn run(instruction: &str, raw: &str) -> Result<InterpretationResult, InterpretationError> {
        interpret_with(&Mock::repeating(raw), instruction)
    }

    fn supported(extensions: &str, actions: &str) -> String {
        format!(
            r#"{{"status":"supported","rule":{{"trigger":{{"type":"file_created","extensions":{extensions}}},"actions":{actions}}},"message":"Preview","clarificationQuestions":[]}}"#
        )
    }

    fn move_action(kind: &str, destination: &str) -> String {
        format!(
            r#"{{"type":"{kind}","destination":"{destination}","requiresFolderSelection":true}}"#
        )
    }

    fn rejection_field(result: Result<InterpretationResult, InterpretationError>) -> String {
        match result.unwrap_err() {
            InterpretationError::SchemaValidation { field, .. }
            | InterpretationError::MalformedOutput { field, .. } => field,
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn regression_case_a_move_new_pdf_to_documents() {
        let json = supported(
            r#"[".pdf"]"#,
            &format!("[{}]", move_action("move", "Documents")),
        );
        let result = run("Move new PDF files to my Documents folder", &json).unwrap();
        assert_eq!(result.rule.unwrap().actions.len(), 1);
    }

    #[test]
    fn regression_case_b_arabic_png_rename_with_date() {
        let json = supported(
            r#"[".png"]"#,
            r#"[{"type":"rename","pattern":"{date}-{originalName}"}]"#,
        );
        let result = run("غير اسمها بتاريخ النهارده لما أنزل صورة PNG", &json).unwrap();
        assert_eq!(result.rule.unwrap().trigger.extensions, [".png"]);
    }

    #[test]
    fn regression_case_c_english_png_rename_with_date() {
        let json = supported(
            r#"[".png"]"#,
            r#"[{"type":"rename","pattern":"{date}-{originalName}"}]"#,
        );
        assert!(run("Rename every new PNG file using today's date", &json).is_ok());
    }

    #[test]
    fn english_and_arabic_file_filters() {
        let english = supported(
            r#"[".jpg"]"#,
            &format!("[{}]", move_action("move", "Photos")),
        );
        let arabic = supported(
            r#"[".png"]"#,
            &format!("[{}]", move_action("move", "الصور")),
        );
        assert!(run("Move new JPG files to Photos", &english).is_ok());
        assert!(run("انقل صور PNG الجديدة إلى الصور", &arabic).is_ok());
    }

    #[test]
    fn move_only_and_copy_only_remain_distinct() {
        let move_only = supported(
            r#"[".pdf"]"#,
            &format!("[{}]", move_action("move", "Documents")),
        );
        let copy_only = supported(
            r#"[".pdf"]"#,
            &format!("[{}]", move_action("copy", "Documents")),
        );
        assert!(run("Move new PDF files to Documents", &move_only).is_ok());
        assert!(run("Copy new PDF files to Documents", &copy_only).is_ok());
    }

    #[test]
    fn explicit_multi_action_order_is_preserved() {
        let json = supported(
            r#"[".txt"]"#,
            r#"[{"type":"rename","pattern":"{date}-{originalName}"},{"type":"copy","destination":"Archive","requiresFolderSelection":true}]"#,
        );
        let result = run(
            "Rename new TXT files using today's date, then copy them to Archive",
            &json,
        )
        .unwrap();
        assert!(matches!(
            result.rule.unwrap().actions[0],
            RuleAction::Rename { .. }
        ));
    }

    #[test]
    fn unsupported_and_ambiguous_requests_are_not_rules() {
        let unsupported = r#"{"status":"unsupported","rule":null,"message":"App launching is unsupported","clarificationQuestions":[]}"#;
        let ambiguous = r#"{"status":"needs_clarification","rule":null,"message":"Choose a destination","clarificationQuestions":["Which folder?"]}"#;
        assert_eq!(
            run("Launch an app when a file is created", unsupported)
                .unwrap()
                .status,
            InterpretationStatus::Unsupported
        );
        assert_eq!(
            run("Move new files somewhere", ambiguous).unwrap().status,
            InterpretationStatus::NeedsClarification
        );
    }

    #[test]
    fn unsupported_request_cannot_be_substituted_with_a_supported_action() {
        let json = supported(
            r#"[".pdf"]"#,
            &format!("[{}]", move_action("move", "Documents")),
        );
        assert_eq!(
            rejection_field(run("Launch an app when a PDF is created", &json)),
            "status"
        );
    }

    #[test]
    fn explicit_filter_mismatch_identifies_field() {
        let json = supported("[]", &format!("[{}]", move_action("move", "Documents")));
        assert_eq!(
            rejection_field(run("Move new PDF files to Documents", &json)),
            "rule.trigger.extensions"
        );
    }

    #[test]
    fn extra_copy_is_rejected_for_move_only_request() {
        let json = supported(
            r#"[".pdf"]"#,
            &format!(
                "[{},{}]",
                move_action("move", "Documents"),
                move_action("copy", "Documents")
            ),
        );
        assert_eq!(
            rejection_field(run("Move new PDF files to Documents", &json)),
            "rule.actions"
        );
    }

    #[test]
    fn date_rename_requires_date_placeholder() {
        let json = supported(
            r#"[".png"]"#,
            r#"[{"type":"rename","pattern":"{originalName}"}]"#,
        );
        assert_eq!(
            rejection_field(run("Rename PNG files using today's date", &json)),
            "rule.actions.pattern"
        );
    }

    #[test]
    fn rename_pattern_must_not_duplicate_original_extension() {
        let json = supported(
            r#"[".png"]"#,
            r#"[{"type":"rename","pattern":"{date}-{originalName}.png"}]"#,
        );
        assert_eq!(
            rejection_field(run("Rename PNG files using today's date", &json)),
            "rule.actions.pattern"
        );
    }

    #[test]
    fn malformed_output_reports_structure_field() {
        assert_eq!(rejection_field(run("Move PDFs", "{")), "$");
    }

    #[test]
    fn unknown_action_is_reported_at_exact_field() {
        let json = r#"{"status":"supported","rule":{"trigger":{"type":"file_created","extensions":[]},"actions":[{"type":"launch","app":"x"}]},"message":"unsupported","clarificationQuestions":[]}"#;
        assert_eq!(
            rejection_field(run("Launch an app", json)),
            "rule.actions[0].type"
        );
    }

    #[test]
    fn malformed_first_response_gets_one_corrective_retry() {
        let corrected = supported(
            r#"[".pdf"]"#,
            &format!("[{}]", move_action("move", "Documents")),
        );
        let mock = Mock::then("{", &corrected);
        assert!(interpret_with(&mock, "Move new PDF files to Documents").is_ok());
        assert_eq!(mock.calls.get(), 2);
        assert!(mock.saw_correction.get());
    }

    #[test]
    fn retry_is_limited_to_one() {
        let mock = Mock::repeating("{");
        assert!(matches!(
            interpret_with(&mock, "Move PDF files"),
            Err(InterpretationError::MalformedOutput { .. })
        ));
        assert_eq!(mock.calls.get(), 2);
    }

    #[test]
    fn empty_and_oversized_instructions_are_rejected_before_ollama() {
        assert!(matches!(
            interpret_instruction("  "),
            Err(InterpretationError::EmptyInstruction)
        ));
        assert!(matches!(
            interpret_instruction(&"x".repeat(MAX_INSTRUCTION_CHARS + 1)),
            Err(InterpretationError::InstructionTooLong)
        ));
    }

    #[test]
    #[ignore = "requires local Ollama and gemma3:4b"]
    fn live_local_model_interpretation() {
        let cases = [
            ("Move new PDF files to my Documents folder", ".pdf", "move"),
            (
                "غير اسمها بتاريخ النهارده لما أنزل صورة PNG",
                ".png",
                "rename",
            ),
            (
                "Rename every new PNG file using today's date",
                ".png",
                "rename",
            ),
        ];
        for (instruction, expected_extension, expected_action) in cases {
            let result = interpret_instruction(instruction).unwrap();
            assert_eq!(result.status, InterpretationStatus::Supported);
            let rule = result.rule.unwrap();
            assert!(
                rule.trigger
                    .extensions
                    .iter()
                    .any(|extension| extension == expected_extension)
            );
            assert_eq!(rule.actions.len(), 1);
            let actual_action = match &rule.actions[0] {
                RuleAction::Move { .. } => "move",
                RuleAction::Copy { .. } => "copy",
                RuleAction::Rename { pattern } => {
                    assert!(pattern.contains("{date}"));
                    assert!(!pattern_has_extension_suffix(pattern));
                    "rename"
                }
            };
            assert_eq!(actual_action, expected_action);
        }
    }
}
