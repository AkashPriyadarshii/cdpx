use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::env;
use std::time::Duration;

const TYPESAFE_URL: &str = "https://api.typesafe.ai/v1/systemone";

#[derive(Debug, thiserror::Error)]
pub enum JevError {
    #[error("Missing TYPESAFE_API_KEY")]
    MissingApiKey,
    #[error("TypeSafe API error: {0}")]
    ApiError(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Request timeout")]
    Timeout,
}

#[derive(Debug, Clone)]
pub struct JevChoice {
    pub choice: String,
    #[allow(dead_code)]
    pub probabilities: Map<String, Value>,
    #[allow(dead_code)]
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct JevNoul {
    pub noul: f64,
}

#[derive(Debug, Clone)]
pub struct JevScore {
    pub score: f64,
    #[allow(dead_code)]
    pub probabilities: Map<String, Value>,
    #[allow(dead_code)]
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JevResponseRaw {
    model: String,
    answers: Map<String, Value>,
    usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct JevRequest {
    pub model: String,
    pub state: Value,
    pub questions: Value,
}

fn get_api_key() -> Result<String, JevError> {
    env::var("TYPESAFE_API_KEY").map_err(|_| JevError::MissingApiKey)
}

pub async fn choice(state: Value, questions: Value) -> Result<JevChoice, JevError> {
    let api_key = get_api_key()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| JevError::ApiError(e.to_string()))?;

    let req = JevRequest {
        model: env::var("TYPESAFE_MODEL").unwrap_or_else(|_| "jev-latest".to_string()),
        state,
        questions,
    };

    let resp = client
        .post(TYPESAFE_URL)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await
        .map_err(|e| JevError::ApiError(e.to_string()))?;

    if resp.status() == 429 {
        return Err(JevError::Timeout);
    }
    if !resp.status().is_success() {
        return Err(JevError::ApiError(format!("HTTP {}", resp.status())));
    }

    let body: JevResponseRaw = resp
        .json()
        .await
        .map_err(|e| JevError::InvalidResponse(e.to_string()))?;
    parse_choice(body)
}

pub async fn noul(state: Value, questions: Value) -> Result<JevNoul, JevError> {
    let api_key = get_api_key()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| JevError::ApiError(e.to_string()))?;

    let req = JevRequest {
        model: env::var("TYPESAFE_MODEL").unwrap_or_else(|_| "jev-latest".to_string()),
        state,
        questions,
    };

    let resp = client
        .post(TYPESAFE_URL)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await
        .map_err(|e| JevError::ApiError(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(JevError::ApiError(format!("HTTP {}", resp.status())));
    }

    let body: JevResponseRaw = resp
        .json()
        .await
        .map_err(|e| JevError::InvalidResponse(e.to_string()))?;
    parse_noul(body)
}

pub async fn score(state: Value, questions: Value) -> Result<JevScore, JevError> {
    let api_key = get_api_key()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| JevError::ApiError(e.to_string()))?;

    let req = JevRequest {
        model: env::var("TYPESAFE_MODEL").unwrap_or_else(|_| "jev-latest".to_string()),
        state,
        questions,
    };

    let resp = client
        .post(TYPESAFE_URL)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await
        .map_err(|e| JevError::ApiError(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(JevError::ApiError(format!("HTTP {}", resp.status())));
    }

    let body: JevResponseRaw = resp
        .json()
        .await
        .map_err(|e| JevError::InvalidResponse(e.to_string()))?;
    parse_score(body)
}

fn parse_choice(body: JevResponseRaw) -> Result<JevChoice, JevError> {
    let answer = body
        .answers
        .get("operation")
        .ok_or_else(|| JevError::InvalidResponse("No operation answer".to_string()))?;
    let choice = answer
        .get("choice")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JevError::InvalidResponse("No choice".to_string()))?
        .to_string();
    let confidence = answer
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let probabilities = answer
        .get("probabilities")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    Ok(JevChoice {
        choice,
        probabilities,
        confidence,
    })
}

fn parse_noul(body: JevResponseRaw) -> Result<JevNoul, JevError> {
    for (_, answer) in &body.answers {
        if let Some(noul) = answer.get("noul").and_then(|v| v.as_f64()) {
            return Ok(JevNoul { noul });
        }
    }
    Err(JevError::InvalidResponse("No noul".to_string()))
}

fn parse_score(body: JevResponseRaw) -> Result<JevScore, JevError> {
    for (_, answer) in &body.answers {
        if let Some(score) = answer.get("score").and_then(|v| v.as_f64()) {
            let confidence = answer
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let probabilities = answer
                .get("probabilities")
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            return Ok(JevScore {
                score,
                probabilities,
                confidence,
            });
        }
    }
    Err(JevError::InvalidResponse("No score".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jev_choice_structure() {
        let c = JevChoice {
            choice: "test".to_string(),
            probabilities: Map::new(),
            confidence: 0.9,
        };
        assert_eq!(c.choice, "test");
        assert_eq!(c.confidence, 0.9);
    }
}
