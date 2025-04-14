// LLM関連のスキーマ定義

// GitHub関連のスキーマ（Git Cloneベース）
pub mod github_response {
    use serde::{Deserialize, Serialize};

    // リポジトリ情報
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RepoInfo {
        pub owner: String,
        pub repo: String,
        pub max_files: usize,
    }

    // ファイル情報
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FileInfo {
        pub path: String,
        pub content: String,
    }
}

// OpenAI API 応答に関するスキーマ
pub mod openai_response {
    use serde::{Deserialize, Serialize};

    // Azureエンドポイント設定
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Endpoint {
        pub name: String,
        pub key: String,
        pub endpoint: String,
    }

    // チャットメッセージ
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ChatMessage {
        pub role: String,
        pub content: String,
    }

    // OpenAI APIレスポンス
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OpenAIResponse {
        pub id: String,
        pub object: String,
        pub created: u64,
        pub model: String,
        pub choices: Vec<OpenAIChoice>,
        pub usage: OpenAIUsage,
    }

    // OpenAI API選択肢
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OpenAIChoice {
        pub index: usize,
        pub message: ChatMessage,
        pub finish_reason: String,
    }

    // OpenAI APIトークン使用量
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OpenAIUsage {
        pub prompt_tokens: usize,
        pub completion_tokens: usize,
        pub total_tokens: usize,
    }

    // レスポンスデータ保存用
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ResponseData {
        pub repo: String,
        pub debate_type: String, 
        pub turn: usize,
        pub timestamp: String,
        pub endpoint: String,
        pub messages: Vec<ChatMessage>,
        pub tokens_used: usize,
    }
}
