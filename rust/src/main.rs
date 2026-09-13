use rein_ch01_helloworld::{chat, load_config, AppError};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let prompt = std::env::args().nth(1).unwrap_or_else(|| "hello".into());
    let result = match load_config() {
        Ok(config) => chat(&config, &prompt).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(text) => {
            println!("{text}");
        }
        Err(error) => {
            eprintln!("失败（{}）：{}", error.kind(), public_reason(&error));
            std::process::exit(1);
        }
    }
}

fn public_reason(error: &AppError) -> String {
    match error {
        rein_ch01_helloworld::AppError::Config(message) => message.clone(),
        rein_ch01_helloworld::AppError::Timeout => "请求超过 30 秒仍未完成".into(),
        rein_ch01_helloworld::AppError::Network => "网络请求失败".into(),
        rein_ch01_helloworld::AppError::ResponseFormat => "响应不是本程序需要的格式".into(),
        rein_ch01_helloworld::AppError::Api(code) => match code.as_str() {
            "invalid_api_key" | "authentication_error" => "检查 API key 和认证配置".into(),
            "permission_denied" => "检查 key 权限和模型访问权限".into(),
            "insufficient_quota" => "检查账户余额和用量额度".into(),
            "rate_limit_exceeded" => "检查限流策略和请求频率".into(),
            "model_not_found" => "检查模型 ID 和端点支持的模型".into(),
            "server_error" => "检查服务状态，稍后人工决定是否重试".into(),
            _ => "服务端拒绝了请求（unknown）".into(),
        },
        rein_ch01_helloworld::AppError::Sdk => "SDK 请求参数或内部状态错误".into(),
    }
}
