use clap::{Parser, Subcommand};
use codex_cli::auto_fix_report;
use codex_cli::doctor;
use codex_cli::llm::CodexClient;
use codex_cli::repo;
use codex_cli::review_json;
use codex_cli::runtime;
use std::env;
use std::fs;
use std::path::PathBuf;

/// CLI 参数入口。
///
/// 说明：
/// - `Review/Refactor/Doc` 是面向本地单文件的辅助命令（主要用于快速试验 prompt）
/// - `PrAutoFix` 是工作流入口：stdout 输出 JSON 给 GitHub Actions 解析，日志走 stderr
#[derive(Parser)]
#[command(name = "codex")]
#[command(about = "Codex Superpowers CLI - 让 Agent 规则在命令行起飞", long_about = None)]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// 子命令集合。
#[derive(Subcommand)]
enum Commands {
    /// 审查单个文件；可选 `--fix` 直接把模型返回的代码块写回文件。
    Review {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long)]
        fix: bool,
    },
    /// 按指定策略对单个文件重构；策略由 prompt 控制（例如 "modularize"）。
    Refactor {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long, default_value = "modularize")]
        strategy: String,
    },
    /// 基于单个文件生成文档（输出到 stdout，不写入文件）。
    Doc {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long, default_value = "api")]
        kind: String,
    },
    /// 针对 PR 的 Gemini Review 执行自动修复（供 GitHub Actions 调用）。
    PrAutoFix {
        #[arg(long)]
        pr_number: u32,
        #[arg(long)]
        review_text: Option<String>,
        #[arg(long)]
        gemini_review: Option<String>,
        #[arg(long)]
        review_json: Option<PathBuf>,
        #[arg(long, default_value = "2")]
        max_rounds: u8,
        #[arg(long, default_value = "false")]
        yes: bool,
        #[arg(long)]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        rules_file: Option<PathBuf>,
        #[arg(long)]
        changelog_path: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        disable_changelog: bool,
        #[arg(long, default_value_t = false)]
        no_pr_comments: bool,
        #[arg(long)]
        pre_skill: Option<String>,
        #[arg(long)]
        pre_skill_pack_root: Option<PathBuf>,
        #[arg(long)]
        pre_skill_input: Option<String>,
        #[arg(long)]
        pre_skill_input_file: Option<PathBuf>,
    },
    /// 不依赖 GitHub PR：对任意本地仓库执行“解析 Review → 生成补丁 → 应用补丁”。
    AutoFixLocal {
        #[arg(long)]
        repo_root: PathBuf,
        #[arg(long)]
        review_text: Option<String>,
        #[arg(long)]
        review_file: Option<PathBuf>,
        #[arg(long)]
        review_json: Option<PathBuf>,
        #[arg(long, default_value = "2")]
        max_rounds: u8,
        #[arg(long, default_value = "false")]
        yes: bool,
        #[arg(long)]
        rules_file: Option<PathBuf>,
        #[arg(long)]
        changelog_path: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        disable_changelog: bool,
    },
    /// 将标准化 Review Markdown 转换为稳定 JSON 主输入。
    ReviewToJson {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// 聚合 pr-auto-fix/auto-fix-local JSON 输出，生成失败样本 Top 5 周报。
    AutoFixWeeklyReport {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// 检查本机安装、PATH、依赖和模型执行器配置是否与源码同步。
    Doctor {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 加载本地 Skill Pack：自动发现 `skills/*/SKILL.md` 并可执行。
    SkillPack {
        #[command(subcommand)]
        command: SkillPackCommands,
    },
}

#[derive(Subcommand)]
enum SkillPackCommands {
    /// 扫描插件根目录下的 skills，列出可用的 SKILL.md。
    List {
        #[arg(long)]
        plugin_root: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 执行某个 SKILL.md（把 AGENTS.md + SKILL.md 注入 system prompt 后调用模型）。
    Run {
        #[arg(long)]
        plugin_root: Option<PathBuf>,
        #[arg(long)]
        skill: String,
        #[arg(long)]
        input: Option<String>,
        #[arg(long)]
        input_file: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Review { path, fix } => {
            let client = CodexClient::new()?;
            let agents_rules = repo::read_agents_rules();
            println!("🕵️  正在启动 Superpowers 审查: {:?}", path);
            let code = fs::read_to_string(path)?;

            // 把仓库的 AGENTS 规则注入 system prompt，减少“误改/越界”概率。
            let system_prompt = format!(
                "你是一个资深架构师，请根据以下 AGENTS.md 规则审查代码：\n\n{}",
                agents_rules
            );
            let user_prompt = format!(
                "请审查以下代码，并指出不符合规则的地方。如果 fix 模式开启，请直接返回修复后的完整代码，并放在代码块中：\n\n```rust\n{}\n```",
                code
            );

            let result = client.call(&system_prompt, &user_prompt).await?;

            if *fix {
                if let Some(fixed_code) = repo::extract_code_block(&result) {
                    fs::write(path, fixed_code)?;
                    println!("✅ 代码已根据 AI 建议自动修复。");
                } else {
                    println!("⚠️ AI 未返回可自动修复的代码块，请查看建议：\n\n{}", result);
                }
            } else {
                println!("🔍 审查报告：\n\n{}", result);
            }
        }
        Commands::Refactor { path, strategy } => {
            let client = CodexClient::new()?;
            let agents_rules = repo::read_agents_rules();
            println!("🔨 正在执行重构 [{}]: {:?}", strategy, path);
            let code = fs::read_to_string(path)?;

            let system_prompt = format!("你是一个重构专家，规则如下：\n\n{}", agents_rules);
            let user_prompt = format!(
                "请按照 '{}' 策略重构以下代码，返回重构后的代码块：\n\n```rust\n{}\n```",
                strategy, code
            );

            let result = client.call(&system_prompt, &user_prompt).await?;
            if let Some(new_code) = repo::extract_code_block(&result) {
                fs::write(path, new_code)?;
                println!("✨ 重构完成。");
            } else {
                println!("⚠️ 未能提取重构后的代码。");
            }
        }
        Commands::Doc { path, kind } => {
            let client = CodexClient::new()?;
            println!("📝 正在生成 {} 文档: {:?}", kind, path);
            let code = fs::read_to_string(path)?;

            let system_prompt = "你是一个技术文档专家，请根据代码生成简洁的 Markdown 文档。";
            let user_prompt = format!(
                "请为以下代码生成 {} 类型的文档：\n\n```rust\n{}\n```",
                kind, code
            );

            let result = client.call(system_prompt, &user_prompt).await?;
            println!("📄 生成的文档内容：\n\n{}", result);
        }
        Commands::PrAutoFix {
            pr_number,
            review_text,
            gemini_review,
            review_json: review_json_path,
            max_rounds,
            yes,
            repo_root,
            rules_file,
            changelog_path,
            disable_changelog,
            no_pr_comments,
            pre_skill,
            pre_skill_pack_root,
            pre_skill_input,
            pre_skill_input_file,
        } => {
            let client = CodexClient::new()?;
            // 日志走 stderr；stdout 只输出 JSON 结果（供 workflow 解析）。
            eprintln!("🤖 启动 PR Auto-Fix #{}", pr_number);
            if review_text.is_some() && gemini_review.is_some() {
                eprintln!("❌ 不能同时提供 --review-text 和 --gemini-review");
                std::process::exit(2);
            }
            if review_text.is_none() && gemini_review.is_none() && review_json_path.is_none() {
                eprintln!("❌ 需要提供 --review-json、--review-text 或 --gemini-review");
                std::process::exit(2);
            }
            let initial_review_data = review_json_path
                .as_deref()
                .map(review_json::read_review_data_file)
                .transpose()?;
            let root = repo_root
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    std::env::var("GITHUB_WORKSPACE").unwrap_or_else(|_| ".".into())
                });

            let mut effective_review = review_text
                .as_ref()
                .or(gemini_review.as_ref())
                .cloned()
                .unwrap_or_default();
            if let Some(pre_skill) = pre_skill.as_deref() {
                if effective_review.is_empty() {
                    eprintln!("❌ --pre-skill 需要 --review-text 或 --gemini-review 作为输入");
                    std::process::exit(2);
                }
                let plugin_root = pre_skill_pack_root
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .or_else(|| env::var("CODEX_SKILL_PACK_ROOT").ok())
                    .or_else(|| {
                        env::current_dir()
                            .ok()
                            .and_then(|d| repo::find_skill_pack_root_from(&d.to_string_lossy()))
                    })
                    .ok_or("无法自动定位 Skill Pack 根目录：请指定 --pre-skill-pack-root 或设置 CODEX_SKILL_PACK_ROOT")?;
                let input_text = pre_skill_input
                    .as_ref()
                    .cloned()
                    .or_else(|| {
                        pre_skill_input_file
                            .as_ref()
                            .and_then(|p| fs::read_to_string(p).ok())
                    })
                    .unwrap_or_else(|| effective_review.clone());

                eprintln!(
                    "🧠 pre-skill: skill={}, plugin_root={}",
                    pre_skill, plugin_root
                );
                effective_review =
                    runtime::run_skill_pack_skill(&plugin_root, pre_skill, &input_text, &client)
                        .await?;
            }

            let rules_file_path = rules_file.as_ref().map(|p| p.to_string_lossy().to_string());
            let changelog_path_str = changelog_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            let options = runtime::AutoFixOptions {
                repo_root: root,
                rules_file: rules_file_path,
                changelog_path: changelog_path_str,
                disable_changelog: *disable_changelog,
                enable_pr_comments: !*no_pr_comments,
            };
            let result = runtime::pr_auto_fix_with_options_and_review_data(
                *pr_number,
                &effective_review,
                *max_rounds,
                *yes,
                options,
                initial_review_data,
                &client,
            )
            .await;
            match result {
                Ok(output) => println!("{}", output),
                Err(e) => {
                    eprintln!("❌ PR Auto-Fix 失败: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::AutoFixLocal {
            repo_root,
            review_text,
            review_file,
            review_json: review_json_path,
            max_rounds,
            yes,
            rules_file,
            changelog_path,
            disable_changelog,
        } => {
            let client = CodexClient::new()?;
            eprintln!(
                "🤖 启动本地 Auto-Fix: repo_root={:?}, review_text={}, review_file={:?}, review_json={:?}",
                repo_root,
                review_text.is_some(),
                review_file,
                review_json_path
            );
            if review_text.is_some() && review_file.is_some() {
                eprintln!("❌ 不能同时提供 --review-text 和 --review-file");
                std::process::exit(2);
            }
            if review_text.is_none() && review_file.is_none() && review_json_path.is_none() {
                eprintln!("❌ 需要提供 --review-json、--review-text 或 --review-file");
                std::process::exit(2);
            }
            let review_text = review_text
                .clone()
                .map(Ok)
                .or_else(|| review_file.as_ref().map(fs::read_to_string))
                .transpose()?
                .unwrap_or_default();
            let initial_review_data = review_json_path
                .as_deref()
                .map(review_json::read_review_data_file)
                .transpose()?;
            let root = repo_root.to_string_lossy().to_string();
            let rules_file_path = rules_file.as_ref().map(|p| p.to_string_lossy().to_string());
            let changelog_path_str = changelog_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            let options = runtime::AutoFixOptions {
                repo_root: root,
                rules_file: rules_file_path,
                changelog_path: changelog_path_str,
                disable_changelog: *disable_changelog,
                enable_pr_comments: false,
            };
            let result = runtime::auto_fix_local_with_review_data(
                &review_text,
                *max_rounds,
                *yes,
                options,
                initial_review_data,
                &client,
            )
            .await;
            match result {
                Ok(output) => println!("{}", output),
                Err(e) => {
                    eprintln!("❌ Auto-Fix-Locally 失败: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::ReviewToJson { input, output } => {
            let json = review_json::convert_review_file(input, output)?;
            print!("{}", json);
        }
        Commands::AutoFixWeeklyReport { input, output } => {
            let raw = fs::read_to_string(input)?;
            let report = auto_fix_report::build_weekly_failure_report(&raw)?;
            fs::write(output, report)?;
            println!("wrote {}", output.display());
        }
        Commands::Doctor { json } => {
            let report = doctor::build_report();
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", doctor::render_human(&report));
            }
        }
        Commands::SkillPack { command } => match command {
            SkillPackCommands::List { plugin_root, json } => {
                let root = plugin_root
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .or_else(|| env::var("CODEX_SKILL_PACK_ROOT").ok())
                    .or_else(|| {
                        env::current_dir()
                            .ok()
                            .and_then(|d| repo::find_skill_pack_root_from(&d.to_string_lossy()))
                    })
                    .ok_or("无法自动定位 Skill Pack 根目录：请指定 --plugin-root")?;
                let skills = repo::discover_skill_pack_skills(&root)?;
                if *json {
                    println!("{}", serde_json::to_string_pretty(&skills)?);
                } else if skills.is_empty() {
                    println!("(empty)");
                } else {
                    for s in skills {
                        let name = s.name.as_deref().unwrap_or(&s.id);
                        let version = s.version.as_deref().unwrap_or("-");
                        println!("{}  v{}  ({})", s.id, version, name);
                    }
                }
            }
            SkillPackCommands::Run {
                plugin_root,
                skill,
                input,
                input_file,
            } => {
                let Some(input_text) = input
                    .as_ref()
                    .cloned()
                    .or_else(|| input_file.as_ref().and_then(|p| fs::read_to_string(p).ok()))
                else {
                    eprintln!("❌ 需要提供 --input 或 --input-file");
                    std::process::exit(1);
                };
                let client = CodexClient::new()?;
                let root = plugin_root
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .or_else(|| env::var("CODEX_SKILL_PACK_ROOT").ok())
                    .or_else(|| {
                        env::current_dir()
                            .ok()
                            .and_then(|d| repo::find_skill_pack_root_from(&d.to_string_lossy()))
                    })
                    .ok_or("无法自动定位 Skill Pack 根目录：请指定 --plugin-root")?;
                let out = runtime::run_skill_pack_skill(&root, skill, &input_text, &client).await?;
                println!("{}", out);
            }
        },
    }

    Ok(())
}
