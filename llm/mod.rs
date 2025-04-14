// LLMコンテキスト情報モジュール
//
// このモジュールは、GitHubリポジトリの分析やAIとの対話に使用される
// プロンプト、テンプレート、スキーマを提供します。

/// APIレスポンスのスキーマ定義
pub mod schemas;

/// システムプロンプトの設定
pub mod prompts;

/// 分析カテゴリと質問
pub mod categories;

/// テンプレートの定義
pub mod templates;
