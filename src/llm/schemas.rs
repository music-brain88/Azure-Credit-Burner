// LLM関連のスキーマ定義

// GitHub API 応答に関するスキーマ
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

    // GitHubのコンテンツAPI応答
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GitHubContent {
        pub name: Option<String>,
        pub path: Option<String>,
        pub sha: String,
        pub size: Option<u64>,
        pub url: String,
        pub html_url: Option<String>,
        pub git_url: Option<String>,
        pub download_url: Option<String>,
        pub r#type: Option<String>,
        pub content: String,
        pub encoding: String,
        pub _links: Option<GitHubLinks>,
    }

    // GitHubのリンク情報
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GitHubLinks {
        pub git: Option<String>,
        pub html: Option<String>,
        pub self_link: Option<String>,
    }

    // GitHubのツリーAPI応答
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GitHubTree {
        pub sha: String,
        pub url: String,
        pub tree: Vec<GitHubTreeItem>,
        pub truncated: Option<bool>,
    }

    // GitHubのツリーアイテム
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GitHubTreeItem {
        pub path: String,
        pub mode: String,
        #[serde(rename = "type")]
        pub item_type: String,
        pub sha: String,
        pub size: Option<u64>,
        pub url: String,
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
