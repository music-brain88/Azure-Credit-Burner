// azure-credit-burner - 3日間で7万ドル分のAzureクレジットを使い切るツール
// Rust版実装

use chrono::prelude::*;
use reqwest::{self, header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::{fs, time};

use anyhow::{Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::Parser;
use dotenv::dotenv;
use futures::{StreamExt, stream};
use log::{error, info};
use simple_logger::SimpleLogger;

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
    #[clap(long, default_value = "25")]
    max_files: usize,
}

// Azure OpenAI のエンドポイント設定
#[derive(Clone, Debug, Serialize, Deserialize)]

struct Endpoint {
    name: String,
    key: String,
    endpoint: String,
}

// GitHubリポジトリ情報
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RepoInfo {
    owner: String,
    repo: String,
    max_files: usize,
}

// ファイル情報
#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileInfo {
    path: String,
    content: String,
}

// チャットメッセージ
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

// 保存用レスポンスデータ
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResponseData {
    repo: String,
    debate_type: String,
    turn: usize,
    timestamp: String,
    endpoint: String,
    messages: Vec<ChatMessage>,
    tokens_used: usize,
}

// GitHub APIレスポンス用
#[derive(Debug, Deserialize)]
struct GitHubTreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    url: Option<String>,
}

// `Clone`トレイトを実装して、所有権を簡単に移動できるようにする
impl Clone for GitHubTreeItem {
    fn clone(&self) -> Self {
        GitHubTreeItem {
            path: self.path.clone(),
            item_type: self.item_type.clone(),
            url: self.url.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubTree {
    tree: Vec<GitHubTreeItem>,
}

#[derive(Debug, Deserialize)]
struct GitHubContent {
    content: String,
    encoding: String,
}

// OpenAI APIレスポンス用
#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

// 深掘り質問カテゴリ
struct DeepQuestions {
    architecture: Vec<String>,
    performance: Vec<String>,
    security: Vec<String>,
    testing: Vec<String>,
    domain: Vec<String>,
    distributed: Vec<String>,
    maintainability: Vec<String>,
}

impl DeepQuestions {
    fn new() -> Self {
        DeepQuestions {
            architecture: vec![

                "このコードベースで使われているデザインパターンを特定し、それらが現代的なベストプラクティスにどう適合または乖離しているか詳細に分析してください。具体的なコード例を引用して説明してください。".to_string(),
                "このプロジェクトのコンポーネント間の依存関係を詳細に分析し、循環依存や密結合が存在する部分を特定してください。その改善案として、どのようなリファクタリングが効果的か具体的に提案してください。".to_string(),
                "このコードベースの拡張性を深く分析してください。新機能追加が困難になりそうなボトルネックはどこにあるか、具体的なコード構造を参照しながら説明し、より柔軟なアーキテクチャへの移行パスを提案してください。".to_string(),
            ],
            performance: vec![
                "このコードベースにおける潜在的なパフォーマンスボトルネックを3つ以上特定し、それぞれがどのような条件下で問題になるか、どの程度のスケールで影響が出始めるかを予測してください。具体的な改善案とその期待効果も詳細に説明してください。".to_string(),
                "メモリ使用量の観点からこのコードを分析し、メモリリークの可能性がある箇所や最適化できる部分を特定してください。大規模データセットで処理する場合、どのようなメモリ最適化戦略が効果的か具体的に提案してください。".to_string(),
                "このコードのマルチスレッド/並列処理の実装を分析し、競合状態やデッドロックの可能性がある箇所を特定してください。並列処理効率を向上させるための具体的なリファクタリング案を、コード例を含めて提案してください。".to_string(),
            ],

            security: vec![

                "このコードベースにOWASPトップ10に関連する脆弱性が存在するか詳細に分析し、具体的なコード箇所を特定してください。各脆弱性に対する修正案を、セキュリティベストプラクティスに基づいて提案してください。".to_string(),
                "認証・認可の実装を詳細に分析し、特権エスカレーションやセッション管理に関する潜在的な脆弱性を特定してください。より堅牢なアクセス制御モデルへの移行計画を具体的に提案してください。".to_string(),
                "データの処理・保存方法からプライバシーとデータ保護の観点で問題となる可能性がある箇所を特定し、GDPR/CCPAなどの規制に準拠するための具体的な改善策を提案してください。".to_string(),
            ],
            testing: vec![
                "現在のテストカバレッジを分析し、重要なビジネスロジックで十分にテストされていない部分を特定してください。優先的に強化すべきテスト領域と、適切なテスト手法（単体/統合/E2E）を提案してください。".to_string(),

                "このコードベースに効果的なプロパティベーステストや変異テストを導入するとしたら、どの部分に適用すべきか分析し、具体的なテスト実装例を3つ以上提案してください。".to_string(),
                "現在のテストの質を分析し、フラキー(不安定)テスト、過度に複雑なテスト、メンテナンスコストの高いテストを特定してください。テスト品質を向上させるためのリファクタリング戦略を詳細に提案してください。".to_string(),
            ],
            domain: vec![

                "このコードベースにおいて、ビジネスロジックとインフラストラクチャの関心事がどの程度分離されているか分析してください。ドメイン駆動設計の原則に基づいて、より明確な境界コンテキストを持つアーキテクチャへのリファクタリング計画を提案してください。".to_string(),
                "このプロダクトの中核的な価値提案（バリュープロポジション）と、それを実現するための重要なコード部分を特定してください。それらの部分が技術的負債やメンテナンス上の課題を抱えていないか詳細に分析してください。".to_string(),
                "このコードベースから、製品が将来的にどのような方向に進化する可能性があるか予測してください。現在のアーキテクチャがその進化をサポートするために必要な変更点を詳細に提案してください。".to_string(),
            ],
            distributed: vec![
                "このシステムをマイクロサービスアーキテクチャに移行するとしたら、どのようなサービス境界が適切か分析し、具体的な分割案と移行戦略を提案してください。各サービスの責任範囲とAPIインターフェースを詳細に説明してください。".to_string(),
                "このシステムの障害耐性と回復力を分析し、単一障害点や耐障害性の低い部分を特定してください。カオスエンジニアリングの原則に基づいて、どのような障害シナリオをテストすべきか、具体的なテスト計画を提案してください。".to_string(),
                "分散トレーシング、集中型ロギング、モニタリングの観点でこのシステムを分析し、オブザーバビリティを向上させるための具体的な改善案を提案してください。どのようなメトリクスやアラートが重要か詳細に説明してください。".to_string(),
            ],
            maintainability: vec![
                "複雑性メトリクス（循環的複雑度、認知的複雑度など）の観点でこのコードベースを分析し、最も複雑な部分を特定してください。これらの部分をより理解しやすく保守しやすくするためのリファクタリング計画を提案してください。".to_string(),
                "このコードにおける命名規則、コメント、ドキュメントの質を詳細に分析し、理解しにくい部分や誤解を招く可能性のある部分を特定してください。読みやすさと保守性を向上させるための具体的な改善案を提案してください。".to_string(),
                "このコードベースにおけるコピー＆ペーストパターンや重複コードを特定し、それらを適切に抽象化・一般化するためのリファクタリング提案を具体的なコード例とともに提示してください。".to_string(),
            ],
        }
    }

    fn get_question(&self, category: &str, index: usize) -> String {
        match category {
            "アーキテクチャ" => self.architecture[index % self.architecture.len()].clone(),
            "パフォーマンス" => self.performance[index % self.performance.len()].clone(),
            "セキュリティ" => self.security[index % self.security.len()].clone(),

            "テスト品質" => self.testing[index % self.testing.len()].clone(),
            "ドメイン分析" => self.domain[index % self.domain.len()].clone(),
            "分散システム" => self.distributed[index % self.distributed.len()].clone(),
            "コード保守性" => {
                self.maintainability[index % self.maintainability.len()].clone()
            }
            _ => "このリポジトリの詳細分析を続けてください。".to_string(),
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
    client: reqwest::Client,

    token: String,
}

impl GitHubClient {
    fn new(token: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github.v3+json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        GitHubClient { client, token }
    }

    async fn fetch_repo_files(&self, repo_info: &RepoInfo) -> Result<Vec<FileInfo>> {
        info!(
            "⬇️ リポジトリからファイル取得中: {}/{}",
            repo_info.owner, repo_info.repo
        );

        // まずmainブランチでファイル一覧を取得
        let mut files_url = format!(
            "https://api.github.com/repos/{}/{}/git/trees/main?recursive=1",
            repo_info.owner, repo_info.repo
        );

        // APIリクエスト用ヘッダー
        let auth_header = format!("token {}", self.token);

        // ファイル一覧を取得

        let mut response = self
            .client
            .get(&files_url)
            .header(header::AUTHORIZATION, &auth_header)
            .send()
            .await;

        // mainブランチが無い場合はmasterを試す
        if response.is_err() || response.as_ref().unwrap().status() != 200 {
            files_url = format!(
                "https://api.github.com/repos/{}/{}/git/trees/master?recursive=1",
                repo_info.owner, repo_info.repo
            );
            response = self
                .client
                .get(&files_url)
                .header(header::AUTHORIZATION, &auth_header)
                .send()
                .await;
        }

        // エラーチェック
        if response.is_err() {
            return Err(anyhow!("リポジトリ情報取得失敗: {:?}", response.err()));
        }

        let response = response.unwrap();
        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!(
                "リポジトリ情報取得エラー: ステータス {}, レスポンス: {:?}",
                status,
                error_text
            ));
        }

        // ファイル一覧をパース
        let tree_data: GitHubTree = response.json().await?;

        // コード関連のファイルをフィルタリング
        let code_extensions = [
            ".py", ".js", ".ts", ".java", ".c", ".cpp", ".h", ".go", ".rs", ".rb", ".php", ".md",
            ".cs", ".jsx", ".tsx",
        ];

        let mut code_files: Vec<GitHubTreeItem> = tree_data
            .tree
            .into_iter()
            .filter(|item| {
                item.item_type == "blob"
                    && code_extensions.iter().any(|&ext| item.path.ends_with(ext))
            })
            .collect();

        // 優先度の高いファイルを先頭に
        code_files.sort_by(|a, b| {
            let a_priority = is_priority_file(&a.path);
            let b_priority = is_priority_file(&b.path);

            if a_priority && !b_priority {
                std::cmp::Ordering::Less
            } else if !a_priority && b_priority {
                std::cmp::Ordering::Greater
            } else {
                a.path.cmp(&b.path)
            }
        });

        // ファイル数を制限
        let max_files = repo_info.max_files.min(code_files.len());
        // 所有権を渡す形に変更
        let selected_files = code_files.into_iter().take(max_files).collect::<Vec<_>>();

        // 各ファイルの内容を並列で取得
        let mut file_infos = Vec::new();
        let branch = if files_url.contains("/main?") {
            "main"
        } else {
            "master"
        };

        // 同時実行数を制限して取得
        let repo_path = format!("{}/{}", repo_info.owner, repo_info.repo);
        let fetched_files = stream::iter(selected_files)
            .map(|file| {
                let client = &self.client;
                let auth = auth_header.clone();
                let repo = repo_path.clone();
                let branch = branch.clone();
                let file_path = file.path.clone(); // クローンして所有権を得る

                async move {
                    // 参照ではなく所有権のある値を使用

                    // ファイル内容のURL構築
                    let content_url = format!(
                        "https://api.github.com/repos/{}/contents/{}",
                        repo, file_path
                    );

                    let response = client
                        .get(&content_url)
                        .header(header::AUTHORIZATION, auth)
                        .query(&[("ref", branch)])
                        .send()
                        .await;

                    match response {
                        Ok(res) => {
                            if res.status() == 200 {
                                match res.json::<GitHubContent>().await {
                                    Ok(content_data) => {
                                        if content_data.encoding == "base64" {
                                            match BASE64
                                                .decode(&content_data.content.replace("\n", ""))
                                            {
                                                Ok(decoded) => {
                                                    let content = String::from_utf8_lossy(&decoded)
                                                        .to_string();

                                                    // 大きなファイルは先頭部分のみ
                                                    let content = if content.len() > 10000 {
                                                        format!(
                                                            "{}...\n(内容省略)...",
                                                            &content[0..10000]
                                                        )
                                                    } else {
                                                        content
                                                    };

                                                    info!("✅ ファイル取得成功: {}", file_path);
                                                    Some(FileInfo {
                                                        path: file_path.clone(),
                                                        content,
                                                    })
                                                }
                                                Err(e) => {
                                                    error!(
                                                        "⚠️ ファイルデコードエラー: {} - {}",
                                                        &file_path, e
                                                    );
                                                    None
                                                }
                                            }
                                        } else {
                                            error!(
                                                "⚠️ 未対応のエンコーディング: {}",
                                                content_data.encoding
                                            );
                                            None
                                        }
                                    }
                                    Err(e) => {
                                        error!("⚠️ ファイル解析エラー: {} - {}", file_path, e);

                                        None
                                    }
                                }
                            } else {
                                error!(
                                    "⚠️ ファイル取得失敗: {} - ステータス: {}",
                                    file_path,
                                    res.status()
                                );
                                None
                            }
                        }
                        Err(e) => {
                            error!("⚠️ ファイルリクエストエラー: {} - {}", file_path, e);
                            None
                        }
                    }
                }
            })
            .buffer_unordered(5) // 同時に5ファイルまで取得
            .collect::<Vec<_>>()
            .await;

        // 取得できたファイルだけを返す
        for file_info_opt in fetched_files {
            if let Some(file_info) = file_info_opt {
                file_infos.push(file_info);
            }
        }

        info!("🗂️ 取得ファイル数: {}/{}", file_infos.len(), max_files);

        if file_infos.is_empty() {
            bail!("リポジトリからファイルを取得できませんでした");
        }

        Ok(file_infos)
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
            api_version: "2023-05-15".to_string(),
        }
    }

    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        model: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<(String, usize)> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint.endpoint, model, self.api_version
        );

        let request_body = json!({
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
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

        // 長すぎる場合は一部を表示
        let content = if file.content.len() > 2000 {
            format!("{}...\n(省略)...", &file.content[0..2000])
        } else {
            file.content.clone()
        };

        file_samples.push_str(&content);
    }

    // システムプロンプト作成
    let system_prompt = format!(
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
        &readme_content[..readme_content.len().min(1000)],
        file_samples,
        debate_type
    );

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
                &messages,
                "gpt-4-32k", // 最大モデルを使用
                4000,        // 長い出力
                0.8,         // 適度な創造性
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
            key: std::env::var("AZURE_OPENAI_KEY_EAST_US").unwrap_or_else(|_| "YOUR_KEY_1".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_EAST_US").unwrap_or_else(|_| "https://eastus.api.cognitive.microsoft.com".to_string()),
        },
        Endpoint {
            name: "west-us".to_string(),
            key: std::env::var("AZURE_OPENAI_KEY_WEST_US").unwrap_or_else(|_| "YOUR_KEY_2".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_WEST_US").unwrap_or_else(|_| "https://westus.api.cognitive.microsoft.com".to_string()),
        },
        Endpoint {
            name: "japan-east".to_string(),
            key: std::env::var("AZURE_OPENAI_KEY_JAPAN_EAST").unwrap_or_else(|_| "YOUR_KEY_3".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_JAPAN_EAST").unwrap_or_else(|_| "https://japaneast.api.cognitive.microsoft.com".to_string()),
        },
        Endpoint {
            name: "europe-west".to_string(),
            key: std::env::var("AZURE_OPENAI_KEY_EUROPE_WEST").unwrap_or_else(|_| "YOUR_KEY_4".to_string()),
            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT_EUROPE_WEST").unwrap_or_else(|_| "https://westeurope.api.cognitive.microsoft.com".to_string()),
        },
    ];

    // GitHubリポジトリ設定を.envから読み込み
    let mut github_repos = Vec::new();
    
    // リポジトリ1
    if let (Ok(owner), Ok(repo)) = (
        std::env::var("REPO_OWNER_1"),
        std::env::var("REPO_NAME_1"),
    ) {
        let max_files = std::env::var("REPO_MAX_FILES_1")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<usize>()
            .unwrap_or(30);
            
        github_repos.push(RepoInfo {
            owner,
            repo,
            max_files,
        });
    }
    
    // リポジトリ2
    if let (Ok(owner), Ok(repo)) = (
        std::env::var("REPO_OWNER_2"),
        std::env::var("REPO_NAME_2"),
    ) {
        let max_files = std::env::var("REPO_MAX_FILES_2")
            .unwrap_or_else(|_| "25".to_string())
            .parse::<usize>()
            .unwrap_or(25);
            
        github_repos.push(RepoInfo {
            owner,
            repo,
            max_files,
        });
    }
    
    // リポジトリ3
    if let (Ok(owner), Ok(repo)) = (
        std::env::var("REPO_OWNER_3"),
        std::env::var("REPO_NAME_3"),
    ) {
        let max_files = std::env::var("REPO_MAX_FILES_3")
            .unwrap_or_else(|_| "20".to_string())
            .parse::<usize>()
            .unwrap_or(20);
            
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
                max_files: 30,
            },
            RepoInfo {
                owner: "your-org".to_string(),
                repo: "your-private-repo2".to_string(),
                max_files: 25,
            },
            RepoInfo {
                owner: "your-org".to_string(),
                repo: "your-private-repo3".to_string(),
                max_files: 20,
            },
        ];
    }

    // 議論タイプ
    let debate_types = get_debate_types();

    // GitHubクライアント (.envまたはコマンドライン引数から)
    let github_token = std::env::var("GITHUB_TOKEN").unwrap_or_else(|_| args.github_token.clone());
    let github_client = Arc::new(GitHubClient::new(github_token));

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
            ).await
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
}
