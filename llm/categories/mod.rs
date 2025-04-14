// 分析カテゴリと質問

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 質問データ
#[derive(Debug, Deserialize, Serialize)]
pub struct Question {
    /// 質問ID
    pub id: String,
    /// 質問テキスト
    pub text: String,
}

/// カテゴリの質問データ
#[derive(Debug, Deserialize, Serialize)]
pub struct CategoryQuestions {
    /// カテゴリ名
    pub category: String,
    /// カテゴリの説明
    pub description: String,
    /// 質問リスト
    pub questions: Vec<Question>,
}

/// カテゴリ別の質問を読み込む
pub fn load_category(category_name: &str) -> Result<CategoryQuestions, String> {
    let base_path = Path::new("llm/categories");
    let file_path = base_path.join(format!("{}.json", category_name));
    
    match fs::read_to_string(file_path) {
        Ok(content) => {
            match serde_json::from_str(&content) {
                Ok(questions) => Ok(questions),
                Err(e) => Err(format!("JSONパースエラー: {}", e)),
            }
        },
        Err(e) => Err(format!("ファイル読み込みエラー: {}", e)),
    }
}

/// 利用可能なカテゴリ一覧を取得
pub fn get_categories() -> Vec<&'static str> {
    vec![
        "architecture",
        "performance",
        "security",
        // 以下のカテゴリは対応するJSONファイルが作成されていない場合はコメントアウト
        // "testing",
        // "domain",
        // "distributed",
        // "maintainability",
    ]
}

/// カテゴリ名を日本語に変換
pub fn get_category_japanese(category: &str) -> &'static str {
    match category {
        "architecture" => "アーキテクチャ",
        "performance" => "パフォーマンス",
        "security" => "セキュリティ",
        "testing" => "テスト品質",
        "domain" => "ドメイン分析",
        "distributed" => "分散システム",
        "maintainability" => "コード保守性",
        _ => "その他",
    }
}

/// カテゴリから質問を取得する便利関数
pub fn get_question(category: &str, index: usize) -> Result<String, String> {
    let category_data = load_category(category)?;
    
    if category_data.questions.is_empty() {
        return Err(format!("カテゴリ「{}」に質問がありません", category));
    }
    
    // 質問のインデックスが範囲外の場合は循環させる
    let question_index = index % category_data.questions.len();
    Ok(category_data.questions[question_index].text.clone())
}
