use serde::{Deserialize, Serialize};

/// GitHubのレスポンス型定義

/// リポジトリツリー内のアイテム
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubTreeItem {
    /// ファイルパス
    pub path: String,
    /// アイテムタイプ（"blob"や"tree"など）
    #[serde(rename = "type")]
    pub item_type: String,
    /// ファイルのURLまたはNone
    pub url: Option<String>,
}

/// リポジトリのファイルツリー情報
#[derive(Debug, Deserialize)]
pub struct GitHubTree {
    /// ツリー内のアイテム一覧
    pub tree: Vec<GitHubTreeItem>,
}

/// GitHubファイルコンテンツ
#[derive(Debug, Deserialize)]
pub struct GitHubContent {
    /// ファイルコンテンツ（通常はBase64エンコードされている）
    pub content: String,
    /// エンコーディング（"base64"など）
    pub encoding: String,
}

/// ファイル情報を格納する構造体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileInfo {
    /// ファイルパス
    pub path: String,
    /// ファイルコンテンツ
    pub content: String,
}

/// リポジトリ情報の構造体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoInfo {
    /// リポジトリオーナー名
    pub owner: String,
    /// リポジトリ名
    pub repo: String,
    /// 処理する最大ファイル数
    pub max_files: usize,
}
