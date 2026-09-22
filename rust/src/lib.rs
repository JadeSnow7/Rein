use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use std::{env, time::Duration};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppError {
    Config(String),
    Timeout,
    Network,
    Sdk,
    ResponseFormat,
    Api(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.kind())
    }
}

impl std::error::Error for AppError {}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Config(_) => "config",
            Self::Timeout => "timeout",
            Self::Network => "network",
            Self::Sdk => "sdk",
            Self::ResponseFormat => "response-format",
            Self::Api(_) => "api",
        }
    }
}

pub fn load_config() -> Result<Config, AppError> {
    load_config_from(|name| env::var(name).ok())
}

pub fn load_config_from<F>(mut get: F) -> Result<Config, AppError>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut required = |name: &str| {
        get(name)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::Config(format!("缺少 {name}")))
    };
    let base_url = required("REIN_BASE_URL")?.trim().to_owned();
    let api_key = required("REIN_API_KEY")?.trim().to_owned();
    let model = required("REIN_MODEL")?.trim().to_owned();
    let parsed = url::Url::parse(&base_url)
        .map_err(|_| AppError::Config("REIN_BASE_URL 不是合法 URL".into()))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AppError::Config(
            "REIN_BASE_URL 需要不含凭据、查询和片段的 http(s) 根地址".into(),
        ));
    }
    Ok(Config {
        base_url: base_url.trim_end_matches('/').to_owned(),
        api_key,
        model,
    })
}

pub async fn chat(config: &Config, prompt: &str) -> Result<String, AppError> {
    chat_with_timeout(config, prompt, DEFAULT_TIMEOUT).await
}

pub async fn chat_with_timeout(
    config: &Config,
    prompt: &str,
    timeout: Duration,
) -> Result<String, AppError> {
    let client = build_client(config);
    let user = ChatCompletionRequestUserMessageArgs::default()
        .content(prompt)
        .build()
        .map_err(|_| AppError::ResponseFormat)?;
    let request = CreateChatCompletionRequestArgs::default()
        .model(&config.model)
        .messages([ChatCompletionRequestMessage::User(user)])
        .build()
        .map_err(|_| AppError::ResponseFormat)?;
    let response = tokio::time::timeout(timeout, client.chat().create(request))
        .await
        .map_err(|_| AppError::Timeout)?
        .map_err(classify_error)?;
    response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
        .ok_or(AppError::ResponseFormat)
}

pub fn build_client(config: &Config) -> Client<OpenAIConfig> {
    Client::with_config(
        OpenAIConfig::new()
            .with_api_key(&config.api_key)
            .with_api_base(&config.base_url),
    )
    .with_backoff(backoff::ExponentialBackoff {
        max_elapsed_time: Some(Duration::ZERO),
        ..Default::default()
    })
}

fn classify_error(error: async_openai::error::OpenAIError) -> AppError {
    match error {
        async_openai::error::OpenAIError::Reqwest(error) => {
            if error.is_timeout() {
                AppError::Timeout
            } else {
                AppError::Network
            }
        }
        async_openai::error::OpenAIError::JSONDeserialize(_) => AppError::ResponseFormat,
        async_openai::error::OpenAIError::ApiError(error) => {
            if error.code.as_deref() == Some("insufficient_quota")
                || error.r#type.as_deref() == Some("insufficient_quota")
            {
                AppError::Api("insufficient_quota".into())
            } else {
                let code = error
                    .code
                    .as_deref()
                    .or(error.r#type.as_deref())
                    .unwrap_or("unknown")
                    .to_owned();
                AppError::Api(code)
            }
        }
        async_openai::error::OpenAIError::InvalidArgument(_)
        | async_openai::error::OpenAIError::FileSaveError(_)
        | async_openai::error::OpenAIError::FileReadError(_)
        | async_openai::error::OpenAIError::StreamError(_) => AppError::Sdk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn response_server(
        body: &'static str,
        status: u16,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let _ = socket.read(&mut request).await.unwrap();
            let reason = if status == 200 { "OK" } else { "Error" };
            let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        (format!("http://{address}"), server)
    }

    fn test_config(base_url: String) -> Config {
        Config {
            base_url,
            api_key: "test-key".into(),
            model: "test-model".into(),
        }
    }

    #[test]
    fn config_requires_all_values_and_strips_trailing_slash() {
        let config = load_config_from(|name| match name {
            "REIN_BASE_URL" => Some("http://localhost:9000///".into()),
            "REIN_API_KEY" => Some("test-key".into()),
            "REIN_MODEL" => Some("test-model".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(config.base_url, "http://localhost:9000");
        assert_eq!(
            load_config_from(|_| None),
            Err(AppError::Config("缺少 REIN_BASE_URL".into()))
        );
        for invalid in [
            "http://",
            "http://user:pass@example.com",
            "http://example.com?token=secret",
            "http://example.com#fragment",
        ] {
            let error = load_config_from(|name| match name {
                "REIN_BASE_URL" => Some(invalid.into()),
                "REIN_API_KEY" => Some(" key ".into()),
                "REIN_MODEL" => Some(" model ".into()),
                _ => None,
            })
            .unwrap_err();
            assert_eq!(error.kind(), "config");
        }
        assert_eq!(
            load_config_from(|name| match name {
                "REIN_BASE_URL" => Some(" http://localhost:9000/ ".into()),
                "REIN_API_KEY" => Some("   ".into()),
                "REIN_MODEL" => Some("model".into()),
                _ => None,
            }),
            Err(AppError::Config("缺少 REIN_API_KEY".into()))
        );
    }

    #[tokio::test]
    async fn local_http_server_receives_one_openai_request() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let size = socket.read(&mut chunk).await.unwrap();
                assert!(size > 0, "mock client closed before sending the request");
                request.extend_from_slice(&chunk[..size]);
                if request
                    .windows(b"\"content\":\"hello\"".len())
                    .any(|window| window == b"\"content\":\"hello\"")
                {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request);
            assert!(request.starts_with("POST /chat/completions HTTP/1.1"));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-key"));
            assert!(request.contains(r#""messages":[{"role":"user","content":"hello"}]"#));
            let body = r#"{"id":"chatcmpl-local","object":"chat.completion","created":1,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"hello back"},"finish_reason":"stop"}],"usage":null}"#;
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let config = Config {
            base_url: format!("http://{address}"),
            api_key: "test-key".into(),
            model: "test-model".into(),
        };
        assert_eq!(
            chat_with_timeout(&config, "hello", Duration::from_secs(2))
                .await
                .unwrap(),
            "hello back"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rate_limit_is_requested_once_and_keeps_provider_code() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&requests);
        let server = tokio::spawn(async move {
            while let Ok(Ok((mut socket, _))) =
                tokio::time::timeout(Duration::from_millis(300), listener.accept()).await
            {
                observed.fetch_add(1, Ordering::SeqCst);
                let mut request = [0_u8; 2048];
                let _ = socket.read(&mut request).await.unwrap();
                let body = r#"{"error":{"message":"slow down","type":"rate_limit_error","code":"rate_limit_exceeded"}}"#;
                let response = format!("HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let config = Config {
            base_url: format!("http://{address}"),
            api_key: "test-key".into(),
            model: "test-model".into(),
        };
        let error = chat_with_timeout(&config, "hello", Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(error, AppError::Api("rate_limit_exceeded".into()));
        server.await.unwrap();
        assert_eq!(requests.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn timeout_returns_without_waiting_for_the_server() {
        use tokio::{io::AsyncWriteExt, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            if let Ok(Ok((mut socket, _))) =
                tokio::time::timeout(Duration::from_millis(500), listener.accept()).await
            {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let _ = socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .await;
            }
        });
        let config = Config {
            base_url: format!("http://{address}"),
            api_key: "test-key".into(),
            model: "test-model".into(),
        };
        assert_eq!(
            chat_with_timeout(&config, "hello", Duration::from_millis(20))
                .await
                .unwrap_err(),
            AppError::Timeout
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn malformed_or_empty_sdk_responses_are_failures() {
        for (body, expected) in [
            ("{", AppError::ResponseFormat),
            (
                r#"{"id":"x","object":"chat.completion","created":1,"model":"m","choices":[],"usage":null}"#,
                AppError::ResponseFormat,
            ),
            (
                r#"{"id":"x","object":"chat.completion","created":1,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"   "},"finish_reason":"stop"}],"usage":null}"#,
                AppError::ResponseFormat,
            ),
        ] {
            let (base_url, server) = response_server(body, 200).await;
            assert_eq!(
                chat_with_timeout(&test_config(base_url), "hello", Duration::from_secs(1))
                    .await
                    .unwrap_err(),
                expected
            );
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn provider_errors_are_classified_without_echoing_messages() {
        for (status, code) in [(429, "rate_limit_exceeded"), (503, "unknown")] {
            let body = if status == 429 {
                r#"{"error":{"message":"secret key should never echo","type":"rate_limit_error","code":"rate_limit_exceeded"}}"#
            } else {
                r#"{"error":{"message":"secret key should never echo","type":"server_error","code":"server_error"}}"#
            };
            let (base_url, server) = response_server(body, status).await;
            assert_eq!(
                chat_with_timeout(&test_config(base_url), "hello", Duration::from_secs(1))
                    .await
                    .unwrap_err(),
                AppError::Api(code.into())
            );
            server.await.unwrap();
        }
    }

    #[test]
    fn status_classification_keeps_key_out_of_public_error() {
        assert_eq!(AppError::Sdk.kind(), "sdk");
        assert_eq!(
            AppError::Config("缺少 REIN_API_KEY".into()).kind(),
            "config"
        );
    }
}
