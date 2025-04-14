// プロンプトテンプレートを管理するモジュール
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// テンプレート変数のタイプ
pub type TemplateVariables = Vec<(String, String)>;

// テンプレートをファイルから読み込む
pub fn load_template(template_name: &str) -> Result<String> {
    // テンプレートディレクトリのパス
    let template_dir = "prompts";
    
    // テンプレートファイルのパス
    let template_path = Path::new(template_dir).join(format!("{}.txt", template_name));
    
    // ファイルが存在するか確認
    if !template_path.exists() {
        return Err(anyhow!("テンプレートファイル {} が見つかりません", template_path.display()));
    }
    
    // テンプレートファイルを読み込む
    let template_content = fs::read_to_string(&template_path)
        .map_err(|e| anyhow!("テンプレートファイル {} の読み込みに失敗: {}", template_path.display(), e))?;
    
    Ok(template_content)
}

// テンプレート内の変数を置換
pub fn render_template(template: &str, variables: &TemplateVariables) -> String {
    let mut rendered = template.to_string();
    
    // 各変数を置換
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        rendered = rendered.replace(&placeholder, value);
    }
    
    rendered
}

// デフォルトテンプレートのマップを取得
pub fn get_default_templates() -> HashMap<String, String> {
    let mut templates = HashMap::new();
    
    // リポジトリ分析用テンプレート
    templates.insert(
        "repo_analysis".to_string(),
        r#"あなたは高度なAIエンジニアとして、GitHubリポジトリ「{{owner}}/{{repo}}」の分析を行います。
このリポジトリについて「{{debate_type}}」という観点から詳細に議論してください。

【リポジトリ情報】
所有者: {{owner}}
リポジトリ名: {{repo}}
ファイル数: {{file_count}}

【ファイル一覧】
{{file_summary}}

【README概要】
{{readme}}

【主要ファイルサンプル】
{{file_samples}}

あなたの任務:

1. このリポジトリのコードを詳細に分析し、「{{debate_type}}」の観点から深く考察してください
2. 技術的な長所・短所を特定し、具体的なコード例を引用してください
3. あなたの専門知識に基づいた改善案や代替アプローチを提案してください
4. 業界のベストプラクティスと比較した評価を行ってください
5. このプロジェクトの将来性や発展方向について予測してください

できるだけ具体的なコード例や技術的詳細に基づいて、深い洞察を提供してください。"#.to_string(),
    );
    
    templates
}

// テンプレートをファイルシステムに保存
pub fn save_default_templates() -> Result<()> {
    let templates = get_default_templates();
    
    // テンプレートディレクトリのパス
    let template_dir = "prompts";
    
    // ディレクトリが存在しない場合は作成
    if !Path::new(template_dir).exists() {
        fs::create_dir_all(template_dir)
            .map_err(|e| anyhow!("テンプレートディレクトリの作成に失敗: {}", e))?;
    }
    
    // 各テンプレートをファイルに保存
    for (name, content) in templates {
        let file_path = Path::new(template_dir).join(format!("{}.txt", name));
        fs::write(&file_path, content)
            .map_err(|e| anyhow!("テンプレートファイル {} の保存に失敗: {}", file_path.display(), e))?;
    }
    
    Ok(())
}
