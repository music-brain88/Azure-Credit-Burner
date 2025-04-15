# 💰 Azure Credit Burner 💰

**3日間で7万ドル分のAzureクレジットをGPT-4分析に使い切るプロジェクト！**

## 🔥 概要

Azure Credit Burnerは、Azure OpenAI Serviceのクレジットを効率的に消費するためのRust製ツールです。
GitHubのプライベートリポジトリをGPT-4に深く分析させることで、短期間で大量のAzureクレジットを使用しながら、実用的で価値のある技術的フィードバックを得ることができます。

<!-- llm:readme=/home/archie/workspace/azure-credit-burner/llm/README.md -->

## ✨ 特徴


- **超並列GPT-4実行**: 複数のAzureリージョンで同時にGPT-4を実行 
- **プライベートリポジトリ分析**: GitHubのプライベートリポジトリをコード解析
- **高度な技術的フィードバック**: アーキテクチャ、パフォーマンス、セキュリティなど多角的な分析
- **Rust製の高性能設計**: 非同期処理とスレッド安全な実装で安定した長時間実行
- **JSON形式で結果保存**: 分析結果をすべて構造化データとして保存


## 📋 要件


- Rust と Cargo (最新版推奨)
- GitHub Personal Access Token (プライベートリポジトリアクセス用)

- Azure OpenAI Service のAPIキー (複数リージョン)
- 大量のAzureクレジット 💸

## 🔧 インストール

```bash
# リポジトリをクローン
git clone https://github.com/your-username/azure-credit-burner.git
cd azure-credit-burner

# 依存関係をインストール & ビルド
cargo build --release
```

## ⚙️ 設定

設定を変更するには、`src/main.rs` ファイル内の以下の部分を編集してください：

```rust
// Azure OpenAIエンドポイント設定
let endpoints = vec![
    Endpoint {
        name: "east-us".to_string(),
        key: "YOUR_KEY_1".to_string(),
        endpoint: "https://eastus.api.cognitive.microsoft.com".to_string(),

    },
    // 他のエンドポイントも追加...
];


// GitHubリポジトリ設定

let github_repos = vec![
    RepoInfo {
        owner: "your-org".to_string(),

        repo: "your-private-repo1".to_string(),

        max_files: 30,
    },
    // 他のリポジトリも追加...
];
```

## 🚀 使用方法

```bash
# GitHubトークンを環境変数に設定

export GITHUB_TOKEN="your-github-token"

# 実行
cargo run --release

# オプションを指定して実行
cargo run --release -- --concurrency 12 --output-dir ./analysis_results
```


### コマンドラインオプション


| オプション | 説明 | デフォルト |
|------------|------|------------|
| `--github-token` | GitHubアクセストークン | 環境変数 `GITHUB_TOKEN` |
| `--output-dir` | 結果保存ディレクトリ | `llm_debates` |

| `--concurrency` | 同時実行数 | `8` |

| `--max-files` | リポジトリあたりの最大ファイル数 | `25` |

## 📊 分析カテゴリ

このツールは以下の観点からコードを分析します：

1. **コードレビュー・分析** - 全体的なコード品質とベストプラクティス
2. **アーキテクチャ評価** - 設計パターンと構造的な強み/弱み
3. **代替アプローチ提案** - より効率的な実装方法
4. **セキュリティ脆弱性検出** - 潜在的なセキュリティリスク
5. **パフォーマンス最適化** - ボトルネックとスケーラビリティ
6. **APIデザイン批評** - インターフェース設計の評価

7. **ロードマップ予測** - 将来的な発展方向

8. **ライセンスと影響分析** - オープンソースとの関係

## 🧠 深掘り質問例

各カテゴリでは、以下のような深い技術的質問を使用します：


```
このコードベースにおける潜在的なパフォーマンスボトルネックを3つ以上特定し、
それぞれがどのような条件下で問題になるか、どの程度のスケールで影響が出始めるかを
予測してください。具体的な改善案とその期待効果も詳細に説明してください。

```

## 📁 出力形式

分析結果は以下の形式のJSONファイルとして保存されます：


```

llm_debates/
└── owner_repo/
    ├── コードレビュー_分析_east-us_turn1_20250414_120145.json
    ├── コードレビュー_分析_east-us_turn2_20250414_120302.json
    ├── アーキテクチャの強み_弱み評価_west-us_turn1_20250414_120145.json
    └── ...
```

## ⚠️ 注意事項


- **APIレート制限**: GitHubとAzure OpenAIのAPIレート制限に注意してください
- **コスト管理**: このツールは意図的に大量のAzureクレジットを消費します！
- **アカウント制限**: 大量のリソース使用によりアカウント停止の可能性があります
- **データプライバシー**: プライベートリポジトリのコードがAIに送信されることに注意


## 💡 ユースケース


- **技術負債の発見**: レガシーコードベースの問題点を特定

- **アーキテクチャレビュー**: 設計上の問題や改善点を発見

- **セキュリティ監査**: 潜在的な脆弱性の早期発見
- **期限切れクレジットの有効活用**: 失効前のAzureクレジットを有効利用


## 🔧 開発

```bash
# 開発環境での実行

cargo run


# テスト実行
cargo test


# リンター実行
cargo clippy
```

## 📝 ライセンス

MIT License

## 🙏 謝辞

このプロジェクトは、大量のAzureクレジットを使い切る必要がある状況から生まれました。
使用しないと失効するクレジットを、価値ある技術的知見に変換するアイデアとして開発されました。
