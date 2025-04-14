use serde::{Deserialize, Serialize};

/// OpenAI / Azure OpenAIのレスポンス型定義

/// チャットメッセージ
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// メッセージの役割（"system", "user", "assistant"など）
    pub role: String,
    /// メッセージの内容
    pub content: String,
}

/// OpenAIのトークン使用量
#[derive(Debug, Deserialize)]
pub struct OpenAIUsage {
    /// 使用された合計トークン数
    pub total_tokens: usize,
}

/// OpenAIのチャット選択肢
#[derive(Debug, Deserialize)]
pub struct OpenAIChoice {
    /// 応答メッセージ
    pub message: ChatMessage,
}

/// OpenAIのチャット完了レスポンス
#[derive(Debug, Deserialize)]
pub struct OpenAIResponse {
    /// レスポンスID
    pub id: String,
    /// 選択肢の配列（通常は1つ）
    pub choices: Vec<OpenAIChoice>,
    /// トークン使用量
    pub usage: OpenAIUsage,
}

/// 会話履歴保存用のレスポンスデータ
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseData {
    /// リポジトリ名（"owner/repo"形式）
    pub repo: String,
    /// 議論のタイプ
    pub debate_type: String,
    /// 会話のターン数
    pub turn: usize,
    /// タイムスタンプ（RFC3339形式）
    pub timestamp: String,
    /// 使用したエンドポイント名
    pub endpoint: String,
    /// メッセージ履歴
    pub messages: Vec<ChatMessage>,
    /// 使用されたトークン数
    pub tokens_used: usize,
}

/// Azure OpenAIのエンドポイント設定
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Endpoint {
    /// エンドポイント名
    pub name: String,
    /// APIキー
    pub key: String,
    /// エンドポイントURL
    pub endpoint: String,
}
