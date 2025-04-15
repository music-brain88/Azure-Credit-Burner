// azure-credit-burner - 3日間で7万ドル分のAzureクレジットを使い切るツール
// Rust版実装

use chrono::prelude::*;
use reqwest::{self, header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{fs, process::Command, time};

use anyhow::{Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::Parser;
use dotenv::dotenv;
use futures::{StreamExt, stream};
use ignore::{Walk, WalkBuilder};
use log::{error, info};
use simple_logger::SimpleLogger;
use walkdir::WalkDir;

// llmディレクトリのスキーマを利用
mod llm;
use llm::categories::{self, get_category_japanese};
use llm::schemas::{
    github_response::{FileInfo, RepoInfo},
    openai_response::{ChatMessage, Endpoint, OpenAIResponse, ResponseData},
};

// コマンドライン引数の定義
#[derive(Parser, Debug)]
#[clap(
    name = "azure-credit-burner",
    about = "GPT-4でAzureクレジットを効率的に消費するツール",
    version = "1.0.0"
)]
struct Args {
    /// GitHubのアクセストークン
    #[clap(long, env = "GITHUB_TOKEN")]
    github_token: String,

    /// 保存先ディレクトリ
    #[clap(long, default_value = "llm_debates")]
    output_dir: String,

    /// 同時実行数
    #[clap(long, default_value = "8")]
    concurrency: usize,

    /// ファイルあたりの最大処理数
    #[clap(long, default_value = "50")]
    max_files: usize,

    /// 最大ファイルサイズ（バイト）
    #[clap(long, default_value = "100000")]
    max_file_size: usize,
}

// 深掘り質問カテゴリ
struct DeepQuestions;

impl DeepQuestions {
    fn new() -> Self {
        DeepQuestions {}
    }

    fn get_question(&self, category: &str, index: usize) -> String {
        // 日本語カテゴリ名から英語カテゴリ名に変換
        let category_en = match category {
            "アーキテクチャ" => "architecture",
            "パフォーマンス" => "performance",
            "セキュリティ" => "security",
            "テスト品質" => "testing",
            "ドメイン分析" => "domain",
            "分散システム" => "distributed",
            "コード保守性" => "maintainability",
            _ => "architecture", // デフォルトはアーキテクチャ
        };

        // カテゴリファイルから質問を取得
        match categories::get_question(category_en, index) {
            Ok(question) => question,
            Err(_) => {
                // エラー時のフォールバック質問
                "このリポジトリについて、さらに詳細な分析を行ってください。コードの品質や設計について特に重要な点は何でしょうか？".to_string()
            }
        }
    }

    fn get_category(&self, turn: usize) -> String {
        let categories = vec![
            "アーキテクチャ",
            "パフォーマンス",
            "セキュリティ",
            "テスト品質",
            "ドメイン分析",
            "分散システム",
            "コード保守性",
        ];
        categories[turn % categories.len()].to_string()
    }
}

// 分析タイプの定義
fn get_debate_types() -> Vec<String> {
    vec![
        "コードレビュー・分析".to_string(),
        "アーキテクチャの強み・弱み評価".to_string(),
        "実装の代替アプローチ提案".to_string(),
        "セキュリティ脆弱性の検出".to_string(),
        "パフォーマンス最適化の提案".to_string(),
        "APIデザインの批評".to_string(),
        "プロジェクトのロードマップ予測".to_string(),
        "ライセンスとオープンソースコミュニティへの影響分析".to_string(),
    ]
}

// GitHubクライアント
struct GitHubClient {
    token: String,
    output_dir: String,
    max_file_size: usize,
}

impl GitHubClient {
    fn new(token: String, output_dir: String, max_file_size: usize) -> Self {
        GitHubClient {
            token,
            output_dir,
            max_file_size,
        }
    }

    // リポジトリをクローンする
    async fn clone_repository(&self, repo_info: &RepoInfo) -> Result<String> {
        let repo_dir = format!(
            "{}/repos/{}_{}",
            self.output_dir, repo_info.owner, repo_info.repo
        );

        // すでにクローン済みかチェック
        if Path::new(&repo_dir).exists() {
            info!(
                "🔄 リポジトリはすでにクローン済み: {}/{}",
                repo_info.owner, repo_info.repo
            );
        } else {
            // ディレクトリ作成
            fs::create_dir_all(Path::new(&repo_dir).parent().unwrap()).await?;

            // git clone コマンド実行
            let clone_url = format!(
                "https://{}@github.com/{}/{}.git",
                self.token, repo_info.owner, repo_info.repo
            );

            info!(
                "🔽 リポジトリをクローン中: {}/{}",
                repo_info.owner, repo_info.repo
            );

            let output = Command::new("git")
                .args(["clone", "--depth", "1", &clone_url, &repo_dir])
                .output()
                .await?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("リポジトリのクローンに失敗: {}", error));
            }

            info!(
                "✅ リポジトリのクローン成功: {}/{}",
                repo_info.owner, repo_info.repo
            );
        }

        Ok(repo_dir)
    }

    // コードファイルを判定する関数
    fn is_code_file(path: &str) -> bool {
        let code_extensions = [
            ".py", ".js", ".ts", ".java", ".c", ".cpp", ".h", ".hpp", ".go", ".rs", ".rb", ".php",
            ".md", ".cs", ".jsx", ".tsx", ".css", ".scss", ".less", ".html", ".xml", ".json",
            ".yaml", ".yml", ".toml", ".sh", ".bash", ".ps1", ".sql", ".graphql", ".proto", ".kt",
            ".swift",
        ];

        code_extensions.iter().any(|&ext| path.ends_with(ext))
    }

    // 除外すべきディレクトリを判定する関数
    fn is_excluded_dir(path: &str) -> bool {
        let excluded_dirs = [
            "/.git/",
            "/node_modules/",
            "/target/",
            "/build/",
            "/dist/",
            "/bin/",
            "/obj/",
            "/.idea/",
            "/.vscode/",
            "/vendor/",
            "/deps/",
            "/_build/",
            "/venv/",
            "/__pycache__/",
        ];

        excluded_dirs.iter().any(|&dir| path.contains(dir))
    }

    // リポジトリファイルを取得
    async fn fetch_repo_files(&self, repo_info: &RepoInfo) -> Result<Vec<FileInfo>> {
        info!(
            "⬇️ リポジトリからファイル取得中: {}/{}",
            repo_info.owner, repo_info.repo
        );

        // リポジトリをクローン
        let repo_dir = self.clone_repository(repo_info).await?;

        // ファイル一覧を取得
        let mut files = Vec::new();

        // ignoreクレートを使ってgitignoreなどを考慮したファイル走査
        let walker = WalkBuilder::new(&repo_dir)
            .standard_filters(true) // .gitignoreを考慮
            .hidden(false) // 隠しファイルも対象に
            .build();

        let mut all_files = Vec::new();

        // ファイルをすべて収集
        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        let path_str = path.to_string_lossy().to_string();

                        // コードファイルかつ除外対象でないファイルのみ
                        if Self::is_code_file(&path_str) && !Self::is_excluded_dir(&path_str) {
                            all_files.push(path.to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    error!("⚠️ ファイル列挙エラー: {}", e);
                }
            }
        }

        // 優先度の高いファイルを先頭に
        all_files.sort_by(|a, b| {
            let a_str = a.to_string_lossy();
            let b_str = b.to_string_lossy();
            let a_priority = is_priority_file(&a_str);
            let b_priority = is_priority_file(&b_str);

            if a_priority && !b_priority {
                std::cmp::Ordering::Less
            } else if !a_priority && b_priority {
                std::cmp::Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        // ファイル数を制限
        let max_files = repo_info.max_files.min(all_files.len());
        let selected_files = all_files.into_iter().take(max_files);

        // ファイル内容を読み込む
        for path in selected_files {
            // 相対パスを取得
            let rel_path = path
                .strip_prefix(&repo_dir)
                .map_err(|e| anyhow!("パス変換エラー: {}", e))?
                .to_string_lossy()
                .to_string();

            // ファイルサイズをチェック
            match fs::metadata(&path).await {
                Ok(metadata) => {
                    // 大きすぎるファイルはスキップ
                    if metadata.len() > self.max_file_size as u64 {
                        info!(
                            "⏩ サイズが大きいためスキップ: {} ({} bytes)",
                            rel_path,
                            metadata.len()
                        );
                        continue;
                    }
                }
                Err(e) => {
                    error!("⚠️ ファイルメタデータ取得エラー: {} - {}", rel_path, e);
                    continue;
                }
            }

            // ファイル内容を読み込む
            match fs::read_to_string(&path).await {
                Ok(content) => {
                    info!("✅ ファイル読み込み成功: {}", rel_path);

                    // 長すぎるファイルは先頭部分のみ
                    let content = if content.len() > self.max_file_size {
                        // 文字単位で処理して安全に切り取る
                        let truncated: String = content.chars().take(self.max_file_size).collect();
                        format!("{}...\n(内容省略)...", truncated)
                    } else {
                        content
                    };

                    files.push(FileInfo {
                        path: rel_path,
                        content,
                    });
                }
                Err(e) => {
                    error!("⚠️ ファイル読み込みエラー: {} - {}", rel_path, e);
                }
            }
        }

        info!("🗂️ 取得ファイル数: {}/{}", files.len(), max_files);

        if files.is_empty() {
            bail!("リポジトリからファイルを取得できませんでした");
        }

        Ok(files)
    }
}

// 優先度の高いファイルかどうかを判定
fn is_priority_file(path: &str) -> bool {
    path.ends_with("README.md")
        || path.contains("main.")
        || path.contains("core.")
        || path.contains("/src/")
            && (path.contains("mod.rs") || path.contains("lib.rs") || path.contains("index."))
}

// Azure OpenAI クライアント
struct AzureOpenAIClient {
    client: reqwest::Client,
    endpoint: Endpoint,
    api_version: String,
}

impl AzureOpenAIClient {
    fn new(endpoint: Endpoint) -> Self {
        let client = reqwest::Client::new();

        AzureOpenAIClient {
            client,
            endpoint,
            api_version: "2024-12-01-preview".to_string(),
        }
    }

    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        model: &str,
        max_tokens: usize, //o1を使う場合はmax_completion_tokensに変更してね
        _temperature: f32, //o1を使う場合はtemperatureが不要
    ) -> Result<(String, usize)> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint.endpoint, model, self.api_version
        );

        let request_body = json!({
            "messages": messages,
            "max_completion_tokens": max_tokens,
            //"temperature": temperature, //o1を使う場合はtemperatureが不要
        });

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.endpoint.key)
            .json(&request_body)
            .send()
            .await?;

        if response.status().is_success() {
            let openai_response: OpenAIResponse = response.json().await?;
            Ok((
                openai_response.choices[0].message.content.clone(),
                openai_response.usage.total_tokens,
            ))
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            Err(anyhow!(
                "OpenAI API エラー: ステータス {}, レスポンス: {}",
                status,
                error_text
            ))
        }
    }
}

// リポジトリ分析用プロンプト生成
fn generate_repo_debate_prompt(
    repo_info: &RepoInfo,
    repo_files: &[FileInfo],
    debate_type: &str,
) -> (String, String) {
    // READMEを探す
    let readme_content = repo_files
        .iter()
        .find(|file| file.path.contains("README.md"))
        .map(|file| &file.content[..])
        .unwrap_or("README.mdが見つかりませんでした。");

    // ファイル一覧のサマリー
    let file_summary = repo_files
        .iter()
        .map(|file| format!("- {}", file.path))
        .collect::<Vec<_>>()
        .join("\n");

    // サンプルファイル
    let mut file_samples = String::new();
    for (i, file) in repo_files.iter().enumerate() {
        if i >= 5 {
            break;
        }

        file_samples.push_str(&format!("\n--- {} ---\n", file.path));

        // 長すぎる場合は一部を表示（文字単位で安全に切り取り）
        let content = if file.content.len() > 2000 {
            // 文字単位で処理して安全に切り取る
            let truncated: String = file.content.chars().take(2000).collect();
            format!("{}...\n(省略)...", truncated)
        } else {
            file.content.clone()
        };

        file_samples.push_str(&content);
    }

    // テンプレート読み込みを試みる
    let system_prompt = match llm::prompts::load_template("repo_analysis") {
        Ok(template) => {
            // テンプレート内の変数を置換
            let variables = vec![
                ("owner".to_string(), repo_info.owner.clone()),
                ("repo".to_string(), repo_info.repo.clone()),
                ("debate_type".to_string(), debate_type.to_string()),
                ("file_count".to_string(), repo_files.len().to_string()),
                ("file_summary".to_string(), file_summary),
                (
                    "readme".to_string(),
                    readme_content.chars().take(1000).collect::<String>(),
                ),
                ("file_samples".to_string(), file_samples),
            ];

            llm::prompts::render_template(&template, &variables)
        }
        Err(_) => {
            // テンプレート読み込みエラー時のフォールバックプロンプト
            format!(
                r#"あなたは高度なAIエンジニアとして、GitHubリポジトリ「{}/{}」の分析を行います。
このリポジトリについて「{}」という観点から詳細に議論してください。

【リポジトリ情報】
所有者: {}
リポジトリ名: {}
ファイル数: {}

【ファイル一覧】
{}

【README概要】
{}

【主要ファイルサンプル】
{}

あなたの任務:

1. このリポジトリのコードを詳細に分析し、「{}」の観点から深く考察してください
2. 技術的な長所・短所を特定し、具体的なコード例を引用してください
3. あなたの専門知識に基づいた改善案や代替アプローチを提案してください
4. 業界のベストプラクティスと比較した評価を行ってください
5. このプロジェクトの将来性や発展方向について予測してください

できるだけ具体的なコード例や技術的詳細に基づいて、深い洞察を提供してください。"#,
                repo_info.owner,
                repo_info.repo,
                debate_type,
                repo_info.owner,
                repo_info.repo,
                repo_files.len(),
                file_summary,
                &readme_content.chars().take(1000).collect::<String>(),
                file_samples,
                debate_type
            )
        }
    };

    // 初期メッセージ
    let initial_message = format!(
        "「{}/{}」リポジトリを「{}」の観点から分析します。まず、このプロジェクトの概要と主要コンポーネントを特定しましょう。",
        repo_info.owner, repo_info.repo, debate_type
    );

    (system_prompt, initial_message)
}

// 次の質問を取得
fn get_next_question(repo_info: &RepoInfo, deep_questions: &DeepQuestions, turn: usize) -> String {
    if turn == 1 {
        return format!(
            "「{}/{}」リポジトリを分析します。まず、このプロジェクトの概要と主要コンポーネントを特定しましょう。",
            repo_info.owner, repo_info.repo
        );
    }

    let category = deep_questions.get_category(turn - 2);
    let question_index = (turn - 2) / 7; // 7カテゴリ

    deep_questions.get_question(&category, question_index)
}

// 保存処理
async fn save_response(
    base_dir: &str,
    repo_info: &RepoInfo,
    debate_type: &str,
    endpoint_name: &str,
    turn: usize,
    messages: &[ChatMessage],
    tokens_used: usize,
) -> Result<String> {
    let repo_dir = format!("{}/{}_{}", base_dir, repo_info.owner, repo_info.repo);

    // ディレクトリがなければ作成
    fs::create_dir_all(&repo_dir).await?;

    // ファイル名を生成
    let now = Utc::now();
    let filename = format!(
        "{}/{}_{}_{}_turn{}.json",
        repo_dir,
        debate_type.replace(" ", "_"),
        endpoint_name,
        turn,
        now.format("%Y%m%d_%H%M%S")
    );

    // 保存データを作成
    let response_data = ResponseData {
        repo: format!("{}/{}", repo_info.owner, repo_info.repo),
        debate_type: debate_type.to_string(),
        turn,
        timestamp: now.to_rfc3339(),
        endpoint: endpoint_name.to_string(),
        messages: messages.to_vec(),
        tokens_used,
    };

    // JSONにして保存
    let json_data = serde_json::to_string_pretty(&response_data)?;
    fs::write(&filename, json_data).await?;

    Ok(filename)
}

// リポジトリ分析の実行
async fn debate_runner(
    github_client: Arc<GitHubClient>,
    endpoints: Arc<Vec<Endpoint>>,
    repo_info: RepoInfo,
    debate_type: String,
    endpoint_index: usize,
    base_dir: String,
) -> Result<()> {
    let endpoint = &endpoints[endpoint_index % endpoints.len()];
    let openai_client = AzureOpenAIClient::new(endpoint.clone());

    info!(
        "[{}] リポジトリ分析開始: {}/{} ({})",
        endpoint.name, repo_info.owner, repo_info.repo, debate_type
    );

    // リポジトリファイルを取得
    let repo_files = match github_client.fetch_repo_files(&repo_info).await {
        Ok(files) => files,
        Err(e) => {
            error!(
                "[{}] リポジトリファイル取得エラー: {}/{} - {}",
                endpoint.name, repo_info.owner, repo_info.repo, e
            );
            return Err(e);
        }
    };

    // 初期プロンプト生成
    let (system_prompt, initial_message) =
        generate_repo_debate_prompt(&repo_info, &repo_files, &debate_type);

    // 会話履歴を保持
    let mut messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: initial_message,
        },
    ];

    // 質問生成用
    let deep_questions = DeepQuestions::new();

    // 会話ループ
    let mut turn = 1;
    while turn <= 20 {
        // 最大20ターンまでに制限
        info!(
            "[{}] 分析実行中: {}/{} ({}) - ターン {}",
            endpoint.name, repo_info.owner, repo_info.repo, debate_type, turn
        );

        // OpenAI APIを呼び出し
        match openai_client
            .chat_completion(
                &messages, "o1", // 最大モデルを使用
                4000, // 長い出力
                0.8,  // 適度な創造性
            )
            .await
        {
            Ok((response, tokens_used)) => {
                // レスポンスを会話履歴に追加
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response,
                });

                // 結果を保存
                match save_response(
                    &base_dir,
                    &repo_info,
                    &debate_type,
                    &endpoint.name,
                    turn,
                    &messages,
                    tokens_used,
                )
                .await
                {
                    Ok(filename) => {
                        info!(
                            "[{}] 保存完了: {} (トークン数: {})",
                            endpoint.name, filename, tokens_used
                        );
                    }
                    Err(e) => {
                        error!(
                            "[{}] 保存エラー: {}/{} - ターン {} - {}",
                            endpoint.name, repo_info.owner, repo_info.repo, turn, e
                        );
                    }
                }

                // 次の質問を生成
                let next_question = get_next_question(&repo_info, &deep_questions, turn);

                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: next_question,
                });

                turn += 1;

                // クレジット消費のためあまり待機しない
                time::sleep(Duration::from_millis(500)).await;
            }
            Err(e) => {
                error!(
                    "[{}] OpenAI API エラー: {}/{} - ターン {} - {}",
                    endpoint.name, repo_info.owner, repo_info.repo, turn, e
                );

                // エラー時は少し待ってリトライ
                time::sleep(Duration::from_secs(5)).await;

                // 3回連続でエラーになったら終了
                if turn > 3 {
                    bail!("OpenAI API 呼び出しに複数回失敗しました。終了します。");
                }
            }
        }
    }

    Ok(())
}

// メイン関数
#[tokio::main]
async fn main() -> Result<()> {
    // .envファイルを読み込み
    dotenv().ok();

    // ロガー初期化
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    // コマンドライン引数を解析
    let args = Args::parse();

    // ベースディレクトリ作成
    // .env設定または引数の値を使用
    let output_dir = std::env::var("OUTPUT_DIR").unwrap_or_else(|_| args.output_dir.clone());
    fs::create_dir_all(&output_dir).await?;

    // Azure OpenAIエンドポイント設定を.envから取得
    let endpoints = vec![
        Endpoint {
            name: "east-us".to_string(),
            key: std::env::var("AZURE_OPENAI_KEY_EAST_US")
                .unwrap_or_else(|_| "YOUR_KEY_1".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_EAST_US")
                .unwrap_or_else(|_| "https://eastus.api.cognitive.microsoft.com".to_string()),
        },
        Endpoint {
            name: "west-us".to_string(),
            key: std::env::var("AZURE_OPENAI_KEY_WEST_US")
                .unwrap_or_else(|_| "YOUR_KEY_2".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_WEST_US")
                .unwrap_or_else(|_| "https://westus.api.cognitive.microsoft.com".to_string()),
        },
        Endpoint {
            name: "japan-east".to_string(),
            key: std::env::var("AZURE_OPENAI_KEY_JAPAN_EAST")
                .unwrap_or_else(|_| "YOUR_KEY_3".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_JAPAN_EAST")
                .unwrap_or_else(|_| "https://japaneast.api.cognitive.microsoft.com".to_string()),
        },
        Endpoint {
            name: "europe-west".to_string(),
            key: std::env::var("AZURE_OPENAI_KEY_EUROPE_WEST")
                .unwrap_or_else(|_| "YOUR_KEY_4".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_EUROPE_WEST")
                .unwrap_or_else(|_| "https://westeurope.api.cognitive.microsoft.com".to_string()),
        },
    ];

    // GitHubリポジトリ設定を.envから読み込み
    let mut github_repos = Vec::new();

    // リポジトリ1
    if let (Ok(owner), Ok(repo)) = (std::env::var("REPO_OWNER_1"), std::env::var("REPO_NAME_1")) {
        let max_files = std::env::var("REPO_MAX_FILES_1")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<usize>()
            .unwrap_or(50);

        github_repos.push(RepoInfo {
            owner,
            repo,
            max_files,
        });
    }

    // リポジトリ2
    if let (Ok(owner), Ok(repo)) = (std::env::var("REPO_OWNER_2"), std::env::var("REPO_NAME_2")) {
        let max_files = std::env::var("REPO_MAX_FILES_2")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<usize>()
            .unwrap_or(50);

        github_repos.push(RepoInfo {
            owner,
            repo,
            max_files,
        });
    }

    // リポジトリ3
    if let (Ok(owner), Ok(repo)) = (std::env::var("REPO_OWNER_3"), std::env::var("REPO_NAME_3")) {
        let max_files = std::env::var("REPO_MAX_FILES_3")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<usize>()
            .unwrap_or(50);

        github_repos.push(RepoInfo {
            owner,
            repo,
            max_files,
        });
    }

    // .envから読み込めなかった場合のデフォルト設定
    if github_repos.is_empty() {
        github_repos = vec![
            RepoInfo {
                owner: "your-org".to_string(),
                repo: "your-private-repo1".to_string(),
                max_files: 50,
            },
            RepoInfo {
                owner: "your-org".to_string(),
                repo: "your-private-repo2".to_string(),
                max_files: 50,
            },
            RepoInfo {
                owner: "your-org".to_string(),
                repo: "your-private-repo3".to_string(),
                max_files: 50,
            },
        ];
    }

    // 議論タイプ
    let debate_types = get_debate_types();

    // GitHubクライアント (.envまたはコマンドライン引数から)
    let github_token = std::env::var("GITHUB_TOKEN").unwrap_or_else(|_| args.github_token.clone());
    let github_client = Arc::new(GitHubClient::new(
        github_token,
        output_dir.clone(),
        args.max_file_size,
    ));

    // Azureエンドポイント
    let endpoints = Arc::new(endpoints);

    // 同時実行数を.envから取得（デフォルトはコマンドライン引数）
    let concurrency = std::env::var("CONCURRENCY")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(args.concurrency);

    // 開始メッセージ
    info!("💰💻 Azure Credit Burner 起動中... 💰💻");
    info!("同時実行数: {}", concurrency);
    info!("対象リポジトリ数: {}", github_repos.len());
    info!("ファイル数上限: {}", args.max_files);
    info!("ファイルサイズ上限: {} バイト", args.max_file_size);

    // タスク作成
    let mut tasks = Vec::new();
    let mut task_index = 0;

    // 各リポジトリと議論タイプの組み合わせでタスクを作成
    // Vec<(RepoInfo, String, usize)>のタプルにして後で処理
    let mut task_configs = Vec::new();

    for (i, repo_info) in github_repos.iter().enumerate() {
        for (j, debate_type) in debate_types.iter().enumerate() {
            // 同じリポジトリでも異なる視点で分析
            let endpoint_index = task_index % endpoints.len();

            // タスク設定を記録
            task_configs.push((repo_info.clone(), debate_type.clone(), endpoint_index));
            task_index += 1;

            // 追加でタスクを作成してクレジット消費を増やす
            if i % 2 == 0 && j % 2 == 0 {
                let extra_endpoint_index = (task_index + 2) % endpoints.len();

                // 追加タスクも記録
                task_configs.push((repo_info.clone(), debate_type.clone(), extra_endpoint_index));
                task_index += 1;
            }
        }
    }

    // 記録したタスク設定を元にタスクを作成
    for (repo_info, debate_type, endpoint_index) in task_configs {
        let github_client_owned = github_client.clone();
        let endpoints_owned = endpoints.clone();
        let output_dir_owned = output_dir.clone();

        tasks.push(tokio::spawn(async move {
            debate_runner(
                github_client_owned,
                endpoints_owned,
                repo_info,
                debate_type,
                endpoint_index,
                output_dir_owned,
            )
            .await
        }));
    }

    // バッファリングして同時実行数を制限
    let mut active_tasks = Vec::new();

    for task in tasks {
        active_tasks.push(task);

        if active_tasks.len() >= concurrency {
            let (completed, _index, remaining) = futures::future::select_all(active_tasks).await;

            // 結果を処理
            match completed {
                Ok(Ok(_)) => {
                    info!("🎉 タスク完了");
                }
                Ok(Err(e)) => {
                    error!("❌ タスクエラー: {}", e);
                }
                Err(e) => {
                    error!("💥 タスク実行エラー: {}", e);
                }
            }

            // 残りのタスクを更新
            active_tasks = remaining;
        }
    }

    // 残りのタスクを完了まで待機
    while !active_tasks.is_empty() {
        let (completed, _index, remaining) = futures::future::select_all(active_tasks).await;

        match completed {
            Ok(Ok(_)) => {
                info!("🎉 タスク完了");
            }
            Ok(Err(e)) => {
                error!("❌ タスクエラー: {}", e);
            }
            Err(e) => {
                error!("💥 タスク実行エラー: {}", e);
            }
        }

        active_tasks = remaining;
    }

    info!("✅ すべてのタスク完了！");

    Ok(())
}

// 設定ファイル用構造体（将来的に外部化する場合用）
#[derive(Serialize, Deserialize)]
struct Config {
    github_token: String,
    output_dir: String,
    endpoints: Vec<Endpoint>,
    repos: Vec<RepoInfo>,
    concurrency: usize,
    max_files: usize,
    max_file_size: usize,
}
