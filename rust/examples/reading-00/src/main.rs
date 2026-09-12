use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Payload {
    text: String,
}

#[derive(Debug, PartialEq)]
enum ReadingError {
    Http(u16),
    Json,
    Shape,
}

impl ReadingError {
    fn kind(&self) -> &'static str {
        match self {
            Self::Http(_) => "http",
            Self::Json => "json",
            Self::Shape => "shape",
        }
    }
}

#[derive(Debug)]
struct LocalResponse {
    status: u16,
    body: String,
}

fn extract_text(response: &LocalResponse) -> Result<String, ReadingError> {
    if !(200..300).contains(&response.status) {
        return Err(ReadingError::Http(response.status));
    }
    let value: Value = serde_json::from_str(&response.body).map_err(|_| ReadingError::Json)?;
    if !value.is_object() {
        return Err(ReadingError::Shape);
    }
    let payload: Payload = serde_json::from_value(value).map_err(|_| ReadingError::Shape)?;
    if payload.text.trim().is_empty() {
        return Err(ReadingError::Shape);
    }
    Ok(payload.text)
}

async fn read_text(response: LocalResponse) -> Result<String, ReadingError> {
    extract_text(&response)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let scenario = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "success".to_owned());
    let response = match scenario.as_str() {
        "http" => LocalResponse {
            status: 500,
            body: "{".to_owned(),
        },
        "json" => LocalResponse {
            status: 200,
            body: "{".to_owned(),
        },
        "shape" => LocalResponse {
            status: 200,
            body: "{}".to_owned(),
        },
        _ => LocalResponse {
            status: 200,
            body: r#"{"text":"你好，Rein"}"#.to_owned(),
        },
    };
    match read_text(response).await {
        Ok(text) => println!("{text}"),
        Err(error) => {
            eprintln!("kind={}", error.kind());
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(status: u16, body: &str) -> LocalResponse {
        LocalResponse {
            status,
            body: body.to_owned(),
        }
    }

    #[tokio::test]
    async fn success_preserves_text_and_allows_extra_fields() {
        assert_eq!(
            read_text(response(200, r#"{"text":"你好，Rein"}"#)).await,
            Ok(String::from("你好，Rein"))
        );
        assert_eq!(
            read_text(response(200, r#"{"text":"你好，Rein","extra":true}"#)).await,
            Ok(String::from("你好，Rein"))
        );
        assert_eq!(
            read_text(response(200, r#"{"text":"  保留两端空白  "}"#)).await,
            Ok(String::from("  保留两端空白  "))
        );
    }

    #[tokio::test]
    async fn distinguishes_http_json_and_shape_errors() {
        assert_eq!(
            read_text(response(500, "{")).await,
            Err(ReadingError::Http(500))
        );
        assert_eq!(read_text(response(200, "{")).await, Err(ReadingError::Json));
        for body in [
            r#"{}"#,
            r#"{"text":null}"#,
            r#"{"text":42}"#,
            r#"{"text":"   "}"#,
            r#"[]"#,
            r#"["text"]"#,
            r#""text""#,
            "null",
            "42",
            r#"{"text":""}"#,
        ] {
            assert_eq!(
                read_text(response(200, body)).await,
                Err(ReadingError::Shape)
            );
        }
    }
}
