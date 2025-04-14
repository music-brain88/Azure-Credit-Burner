// システムプロンプトの設定

use std::fs;
use std::path::Path;

/// システムプロンプトを読み込む
pub fn load_system_prompt(prompt_name: &str) -> Result<String, String> {
    let base_path = Path::new("llm/system_prompts");
    let file_path = base_path.join(format!("{}.md", prompt_name));
    
    match fs::read_to_string(file_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("プロンプトファイル読み込みエラー: {}", e)),
    }
}

/// テンプレートを読み込む
pub fn load_template(template_name: &str) -> Result<String, String> {
    let base_path = Path::new("llm/templates");
    let file_path = base_path.join(format!("{}.md", template_name));
    
    match fs::read_to_string(file_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("テンプレートファイル読み込みエラー: {}", e)),
    }
}

/// テンプレート内の変数を置換する
pub fn render_template(template: &str, variables: &[(String, String)]) -> String {
    let mut result = template.to_string();
    
    for (key, value) in variables {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    result
}
