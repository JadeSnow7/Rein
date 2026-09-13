use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use rein_ch01_helloworld::{build_client, load_config, AppError};
use std::time::Duration;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let result = async {
        let config = load_config()?;
        let client = build_client(&config);
        let message = ChatCompletionRequestUserMessageArgs::default()
            .content("hello")
            .build()
            .map_err(|_| AppError::Sdk)?;
        let request = CreateChatCompletionRequestArgs::default()
            .model(&config.model)
            .messages([ChatCompletionRequestMessage::User(message)])
            .build()
            .map_err(|_| AppError::Sdk)?;
        let response = tokio::time::timeout(Duration::from_secs(30), client.chat().create(request))
            .await
            .map_err(|_| AppError::Timeout)?
            .map_err(|_| AppError::Network)?;
        response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_deref())
            .filter(|text| !text.trim().is_empty())
            .ok_or(AppError::ResponseFormat)
            .map(str::to_owned)
    }
    .await;
    match result {
        Ok(text) => println!("{text}"),
        Err(_error) => {
            eprintln!("调用失败，请检查配置、网络和服务端响应；需要分类诊断时运行安全入口");
            std::process::exit(1);
        }
    }
}
