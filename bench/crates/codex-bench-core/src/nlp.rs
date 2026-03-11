use std::collections::{BTreeMap, BTreeSet};

use once_cell::sync::Lazy;
use regex::Regex;
use rust_stemmers::{Algorithm, Stemmer};
use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Deserialize)]
struct ResearchLexicons {
    function_words: Vec<String>,
    hedge_words: Vec<String>,
    certainty_words: Vec<String>,
    planning_words: Vec<String>,
    verification_words: Vec<String>,
    tool_action_words: Vec<String>,
    modal_words: Vec<String>,
    sequencing_words: Vec<String>,
    first_person_singular: Vec<String>,
    first_person_plural: Vec<String>,
    second_person: Vec<String>,
    artifact_reference_words: Vec<String>,
    code_artifact_words: Vec<String>,
    social_alignment_phrases: Vec<String>,
    reflective_phrases: Vec<String>,
    formality_markers: Vec<String>,
    task_words: Vec<String>,
    observation_words: Vec<String>,
    decision_words: Vec<String>,
    result_words: Vec<String>,
    recap_words: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MessageNlpAnalysis {
    pub surface_tokens: Vec<String>,
    pub stem_tokens: Vec<String>,
    pub text_chars: usize,
    pub text_tokens_est: i64,
    pub sentence_count: usize,
    pub paragraph_count: usize,
    pub bullet_count: usize,
    pub codeblock_count: usize,
    pub avg_sentence_length: i64,
    pub question_count: usize,
    pub exclamation_count: usize,
    pub colon_count: usize,
    pub backtick_span_count: usize,
    pub content_word_count: usize,
    pub function_word_count: usize,
    pub type_token_ratio_bps: i64,
    pub lexical_diversity_score_bps: i64,
    pub hapax_ratio_bps: i64,
    pub top_surface_terms: Vec<String>,
    pub top_lemmas: Vec<String>,
    pub top_bigrams: Vec<String>,
    pub top_trigrams: Vec<String>,
    pub hedge_word_count: usize,
    pub certainty_word_count: usize,
    pub planning_verb_count: usize,
    pub verification_verb_count: usize,
    pub tool_action_verb_count: usize,
    pub social_alignment_phrase_count: usize,
    pub first_person_singular_count: usize,
    pub first_person_plural_count: usize,
    pub second_person_count: usize,
    pub modal_verb_count: usize,
    pub sequencing_cue_count: usize,
    pub artifact_reference_count: usize,
    pub code_reference_count: usize,
    pub hedging_score_bps: i64,
    pub confidence_score_bps: i64,
    pub collaboration_tone_score_bps: i64,
    pub directive_score_bps: i64,
    pub reflective_score_bps: i64,
    pub formality_score_bps: i64,
    pub empathy_alignment_score_bps: i64,
    pub bridge_language_score_bps: i64,
    pub verification_language_score_bps: i64,
    pub state_externalization_score_bps: i64,
    pub readability_ease: i64,
    pub readability_grade_bps: i64,
    pub categories: Vec<String>,
    pub contains_question: bool,
    pub contains_uncertainty: bool,
    pub contains_next_step: bool,
    pub contains_tool_intent: bool,
    pub contains_verification_language: bool,
    pub contains_result_claim: bool,
    pub contains_empathy_or_alignment_language: bool,
}

static STEMMER: Lazy<Stemmer> = Lazy::new(|| Stemmer::create(Algorithm::English));
static TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?u)[\p{L}\p{N}]+(?:['’][\p{L}\p{N}]+)*").expect("valid token regex")
});
static LEXICONS: Lazy<ResearchLexicons> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../../../studies/nlp/english-research-lexicons.json"))
        .expect("valid research lexicons")
});

pub fn analyze_message(text: &str, phase: &str) -> MessageNlpAnalysis {
    let normalized = text.replace('’', "'");
    let surface_tokens = tokenize_surface_words(&normalized);
    let stem_tokens = surface_tokens
        .iter()
        .map(|token| STEMMER.stem(token).to_string())
        .collect::<Vec<_>>();

    let text_chars = text.chars().count();
    let sentence_count = count_sentences(text);
    let paragraph_count = count_paragraphs(text);
    let bullet_count = count_bullets(text);
    let codeblock_count = text.matches("```").count() / 2;
    let question_count = text.matches(['?', '？']).count();
    let exclamation_count = text.matches(['!', '！']).count();
    let colon_count = text.matches([':', '：']).count();
    let backtick_span_count = text.matches('`').count() / 2;
    let text_tokens_est = estimate_text_tokens(text);
    let avg_sentence_length = if sentence_count == 0 {
        0
    } else {
        (text_chars / sentence_count.max(1)) as i64
    };

    let content_word_count = count_content_words(&surface_tokens);
    let function_word_count = surface_tokens.len().saturating_sub(content_word_count);
    let type_token_ratio_bps = lexical_diversity_bps(&surface_tokens);
    let lexical_diversity_score_bps = lexical_diversity_bps(&stem_tokens);
    let hapax_ratio_bps = hapax_ratio_bps(&stem_tokens);

    let hedge_word_count = count_tokens_in_set(&surface_tokens, &LEXICONS.hedge_words);
    let certainty_word_count = count_tokens_in_set(&surface_tokens, &LEXICONS.certainty_words);
    let planning_verb_count = count_tokens_in_set(&surface_tokens, &LEXICONS.planning_words);
    let verification_verb_count =
        count_tokens_in_set(&surface_tokens, &LEXICONS.verification_words);
    let tool_action_verb_count =
        count_tokens_in_set(&surface_tokens, &LEXICONS.tool_action_words);
    let social_alignment_phrase_count =
        count_phrase_matches(&normalized, &LEXICONS.social_alignment_phrases);
    let first_person_singular_count =
        count_tokens_in_set(&surface_tokens, &LEXICONS.first_person_singular);
    let first_person_plural_count =
        count_tokens_in_set(&surface_tokens, &LEXICONS.first_person_plural);
    let second_person_count = count_tokens_in_set(&surface_tokens, &LEXICONS.second_person);
    let modal_verb_count = count_tokens_in_set(&surface_tokens, &LEXICONS.modal_words);
    let sequencing_cue_count =
        count_tokens_in_set(&surface_tokens, &LEXICONS.sequencing_words);
    let artifact_reference_count =
        count_tokens_in_set(&surface_tokens, &LEXICONS.artifact_reference_words);
    let code_reference_count =
        count_tokens_in_set(&surface_tokens, &LEXICONS.code_artifact_words)
            + backtick_span_count;

    let contains_question = question_count > 0;
    let contains_uncertainty = hedge_word_count > 0;
    let contains_next_step = sequencing_cue_count > 0
        || contains_any(&normalized, &["next", "then", "i'll", "i will", "now i'm", "going to"]);
    let contains_tool_intent = tool_action_verb_count > 0;
    let contains_verification_language = verification_verb_count > 0;
    let contains_result_claim =
        count_tokens_in_set(&surface_tokens, &LEXICONS.result_words) > 0;
    let contains_empathy_or_alignment_language = social_alignment_phrase_count > 0
        || first_person_plural_count > 0
        || contains_any(&normalized, &["let's", "together"]);

    let hedging_score_bps = ratio_bps(hedge_word_count, surface_tokens.len());
    let confidence_score_bps = ratio_bps(certainty_word_count, surface_tokens.len());
    let collaboration_tone_score_bps = ratio_bps(
        social_alignment_phrase_count + first_person_plural_count,
        sentence_count.max(1),
    );
    let directive_score_bps =
        ratio_bps(planning_verb_count + tool_action_verb_count + modal_verb_count, sentence_count.max(1));
    let reflective_score_bps =
        ratio_bps(count_phrase_matches(&normalized, &LEXICONS.reflective_phrases), sentence_count.max(1));
    let formality_score_bps =
        ratio_bps(count_phrase_matches(&normalized, &LEXICONS.formality_markers), sentence_count.max(1));
    let empathy_alignment_score_bps =
        ratio_bps(social_alignment_phrase_count + first_person_plural_count, sentence_count.max(1));
    let bridge_language_score_bps = ratio_bps(
        sequencing_cue_count + tool_action_verb_count + artifact_reference_count,
        surface_tokens.len().max(1),
    );
    let verification_language_score_bps = ratio_bps(
        verification_verb_count + count_tokens_in_set(&surface_tokens, &LEXICONS.result_words),
        surface_tokens.len().max(1),
    );
    let state_externalization_score_bps = ratio_bps(
        count_tokens_in_set(&surface_tokens, &LEXICONS.observation_words)
            + count_tokens_in_set(&surface_tokens, &LEXICONS.decision_words)
            + planning_verb_count
            + verification_verb_count,
        surface_tokens.len().max(1),
    );

    let categories = classify_discourse(
        &normalized,
        phase,
        &surface_tokens,
        contains_next_step,
        contains_tool_intent,
        contains_verification_language,
        contains_result_claim,
        contains_empathy_or_alignment_language,
    );

    MessageNlpAnalysis {
        surface_tokens: surface_tokens.clone(),
        stem_tokens: stem_tokens.clone(),
        text_chars,
        text_tokens_est,
        sentence_count,
        paragraph_count,
        bullet_count,
        codeblock_count,
        avg_sentence_length,
        question_count,
        exclamation_count,
        colon_count,
        backtick_span_count,
        content_word_count,
        function_word_count,
        type_token_ratio_bps,
        lexical_diversity_score_bps,
        hapax_ratio_bps,
        top_surface_terms: top_terms_for_message(&surface_tokens, 6),
        top_lemmas: top_terms_for_message(&stem_tokens, 6),
        top_bigrams: top_terms_for_message(&make_ngrams_from_tokens(&stem_tokens, 2), 4),
        top_trigrams: top_terms_for_message(&make_ngrams_from_tokens(&stem_tokens, 3), 4),
        hedge_word_count,
        certainty_word_count,
        planning_verb_count,
        verification_verb_count,
        tool_action_verb_count,
        social_alignment_phrase_count,
        first_person_singular_count,
        first_person_plural_count,
        second_person_count,
        modal_verb_count,
        sequencing_cue_count,
        artifact_reference_count,
        code_reference_count,
        hedging_score_bps,
        confidence_score_bps,
        collaboration_tone_score_bps,
        directive_score_bps,
        reflective_score_bps,
        formality_score_bps,
        empathy_alignment_score_bps,
        bridge_language_score_bps,
        verification_language_score_bps,
        state_externalization_score_bps,
        readability_ease: flesch_reading_ease(&normalized, sentence_count, surface_tokens.len()),
        readability_grade_bps: flesch_kincaid_grade_bps(&normalized, sentence_count, surface_tokens.len()),
        categories,
        contains_question,
        contains_uncertainty,
        contains_next_step,
        contains_tool_intent,
        contains_verification_language,
        contains_result_claim,
        contains_empathy_or_alignment_language,
    }
}

pub fn tokenize_research_terms(text: &str) -> Vec<String> {
    tokenize_surface_words(&text.to_lowercase())
        .into_iter()
        .map(|token| STEMMER.stem(&token).to_string())
        .filter(|token| token.len() >= 3)
        .filter(|token| !token.chars().any(|ch| ch.is_ascii_digit()))
        .filter(|token| !token.contains("users") && !token.contains("downloads") && !token.contains("codexplusclaw"))
        .filter(|token| !LEXICONS.function_words.iter().any(|stop| stop == token))
        .filter(|token| {
            !matches!(
                token.as_str(),
                "workspace"
                    | "artifact"
                    | "artifacts"
                    | "run"
                    | "attempt"
                    | "path"
                    | "file"
                    | "swebench"
                    | "studi"
                    | "kevinlin"
                    | "friendli"
                    | "pragmat"
                    | "gpt"
                    | "codex"
                    | "model"
            )
        })
        .collect()
}

fn classify_discourse(
    normalized: &str,
    phase: &str,
    tokens: &[String],
    contains_next_step: bool,
    contains_tool_intent: bool,
    contains_verification_language: bool,
    contains_result_claim: bool,
    contains_empathy_or_alignment_language: bool,
) -> Vec<String> {
    let mut categories = Vec::new();
    if phase == "commentary" {
        categories.push("orientation".to_string());
    }
    if count_tokens_in_set(tokens, &LEXICONS.task_words) > 0 {
        categories.push("task_restatement".to_string());
    }
    if contains_next_step || count_tokens_in_set(tokens, &LEXICONS.planning_words) > 0 {
        categories.push("planning".to_string());
    }
    if count_tokens_in_set(tokens, &LEXICONS.observation_words) > 0 {
        categories.push("observation".to_string());
    }
    if count_tokens_in_set(tokens, &LEXICONS.decision_words) > 0
        || contains_any(normalized, &["because", "so that", "which means", "in order to"])
    {
        categories.push("decision_explanation".to_string());
    }
    if contains_tool_intent && contains_next_step {
        categories.push("tool_bridge_before".to_string());
    }
    if contains_any(
        normalized,
        &["the result", "output shows", "that means", "this confirms", "this suggests"],
    ) {
        categories.push("tool_bridge_after".to_string());
    }
    if contains_verification_language {
        categories.push("verification_framing".to_string());
    }
    if phase == "finalanswer" || contains_result_claim {
        categories.push("result_framing".to_string());
    }
    if contains_empathy_or_alignment_language {
        categories.push("social_tone".to_string());
    }
    if count_tokens_in_set(tokens, &LEXICONS.recap_words) > 0 {
        categories.push("redundant_recap".to_string());
    }
    if categories.is_empty() {
        categories.push("observation".to_string());
    }
    categories.sort();
    categories.dedup();
    categories
}

fn tokenize_surface_words(text: &str) -> Vec<String> {
    TOKEN_RE
        .find_iter(text)
        .map(|m| m.as_str().trim_matches('\'').to_ascii_lowercase())
        .filter(|token| !token.is_empty())
        .collect()
}

fn count_content_words(tokens: &[String]) -> usize {
    tokens
        .iter()
        .filter(|token| !LEXICONS.function_words.iter().any(|stop| stop == token.as_str()))
        .count()
}

fn count_tokens_in_set(tokens: &[String], needles: &[String]) -> usize {
    let set = needles.iter().map(String::as_str).collect::<BTreeSet<_>>();
    tokens.iter().filter(|token| set.contains(token.as_str())).count()
}

fn count_phrase_matches(text: &str, phrases: &[String]) -> usize {
    let lowered = text.to_ascii_lowercase();
    phrases
        .iter()
        .map(|phrase| lowered.matches(&phrase.to_ascii_lowercase()).count())
        .sum()
}

fn lexical_diversity_bps(tokens: &[String]) -> i64 {
    if tokens.is_empty() {
        return 0;
    }
    let unique = tokens.iter().collect::<BTreeSet<_>>().len();
    ((unique as i64) * 10_000) / (tokens.len() as i64)
}

fn hapax_ratio_bps(tokens: &[String]) -> i64 {
    if tokens.is_empty() {
        return 0;
    }
    let mut counts = BTreeMap::<&str, usize>::new();
    for token in tokens {
        *counts.entry(token.as_str()).or_default() += 1;
    }
    let hapax = counts.values().filter(|count| **count == 1).count();
    ((hapax as i64) * 10_000) / (tokens.len() as i64)
}

fn ratio_bps(numerator: usize, denominator: usize) -> i64 {
    if denominator == 0 {
        0
    } else {
        (((numerator as i64) * 10_000) / (denominator as i64)).clamp(0, 10_000)
    }
}

fn top_terms_for_message(tokens: &[String], limit: usize) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for token in tokens {
        *counts.entry(token.clone()).or_default() += 1;
    }
    let mut entries = counts.into_iter().collect::<Vec<_>>();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    entries
        .into_iter()
        .take(limit)
        .map(|(term, count)| format!("{term}:{count}"))
        .collect()
}

fn make_ngrams_from_tokens(tokens: &[String], n: usize) -> Vec<String> {
    if tokens.len() < n {
        return Vec::new();
    }
    tokens.windows(n).map(|window| window.join(" ")).collect()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lowered = text.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| lowered.contains(&needle.to_ascii_lowercase()))
}

fn estimate_text_tokens(text: &str) -> i64 {
    let chars = text.chars().count() as i64;
    (chars / 4).max(1)
}

fn count_sentences(text: &str) -> usize {
    text.matches(['.', '!', '?', '。', '！', '？'])
        .count()
        .max(usize::from(!text.trim().is_empty()))
}

fn count_paragraphs(text: &str) -> usize {
    text.split("\n\n")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .count()
        .max(usize::from(!text.trim().is_empty()))
}

fn count_bullets(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("1. ")
                || trimmed.starts_with("2. ")
                || trimmed.starts_with("3. ")
        })
        .count()
}

fn count_syllables(word: &str) -> usize {
    let lowered = word.to_ascii_lowercase();
    let chars = lowered.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return 0;
    }
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let mut count = 0usize;
    let mut prev_vowel = false;
    for ch in chars.iter().copied() {
        let is_vowel = vowels.contains(&ch);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }
    if lowered.ends_with('e') && count > 1 {
        count -= 1;
    }
    count.max(1)
}

fn flesch_reading_ease(text: &str, sentence_count: usize, word_count: usize) -> i64 {
    if sentence_count == 0 || word_count == 0 {
        return 0;
    }
    let syllables = tokenize_surface_words(text)
        .iter()
        .map(|word| count_syllables(word))
        .sum::<usize>() as f64;
    let words = word_count as f64;
    let sentences = sentence_count as f64;
    let score = 206.835 - 1.015 * (words / sentences) - 84.6 * (syllables / words);
    score.round() as i64
}

fn flesch_kincaid_grade_bps(text: &str, sentence_count: usize, word_count: usize) -> i64 {
    if sentence_count == 0 || word_count == 0 {
        return 0;
    }
    let syllables = tokenize_surface_words(text)
        .iter()
        .map(|word| count_syllables(word))
        .sum::<usize>() as f64;
    let words = word_count as f64;
    let sentences = sentence_count as f64;
    let score = 0.39 * (words / sentences) + 11.8 * (syllables / words) - 15.59;
    (score * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_message_extracts_richer_language_profile() {
        let analysis = analyze_message(
            "Let's verify the fix, then run the regression test and explain the result clearly.",
            "commentary",
        );
        assert!(analysis.surface_tokens.iter().any(|token| token == "verify"));
        assert!(analysis.top_lemmas.iter().any(|term| term.starts_with("verifi:")));
        assert!(analysis.categories.contains(&"planning".to_string()));
        assert!(analysis.categories.contains(&"verification_framing".to_string()));
        assert!(analysis.categories.contains(&"social_tone".to_string()));
        assert!(analysis.bridge_language_score_bps > 0);
        assert!(analysis.state_externalization_score_bps > 0);
    }

    #[test]
    fn tokenize_research_terms_stems_and_filters_noise() {
        let tokens = tokenize_research_terms(
            "Codex friendly runs verify serializers while artifacts and workspace noise are ignored.",
        );
        assert!(tokens.contains(&"verifi".to_string()));
        assert!(tokens.contains(&"serial".to_string()));
        assert!(!tokens.contains(&"friendli".to_string()));
        assert!(!tokens.contains(&"artifact".to_string()));
    }
}
