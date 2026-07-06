//! wechat-emma: run multiple isolated WeChat instances on macOS.

rust_i18n::i18n!("locales", fallback = "en");

pub mod banner;
pub mod cli;
pub mod commands;
pub mod config;
pub mod data;
pub mod error;
pub mod help;
pub mod i18n;
pub mod instance;
pub mod output;
pub mod plist_edit;
pub mod sysops;

use std::path::PathBuf;
use std::process::ExitCode;

/// Extract the value of a `--lang` flag from raw args, for locale resolution
/// before clap parsing runs (so clap's own errors can be localized).
fn lang_from_args<I: Iterator<Item = String>>(args: I) -> Option<String> {
    let args: Vec<String> = args.collect();
    for (i, a) in args.iter().enumerate() {
        if let Some(v) = a.strip_prefix("--lang=") {
            return Some(v.to_string());
        }
        if a == "--lang" {
            return args.get(i + 1).cloned();
        }
    }
    None
}

/// Parse arguments, dispatch, and map the outcome to a process exit code.
pub fn run() -> ExitCode {
    use clap::Parser;

    // Resolve locale from raw args + config BEFORE clap parsing, so we can render
    // clap's own errors (missing args, unknown command, help) in the right
    // language even when parsing fails.
    let cfg = config::Config::load_from(&config::default_config_path()).unwrap_or_default();
    let flag_lang = lang_from_args(std::env::args());
    let locale = i18n::resolve_locale(
        flag_lang.as_deref(),
        cfg.lang.as_deref(),
        std::env::var("LANG").ok().as_deref(),
    );
    i18n::apply(locale);

    // Parse, but intercept clap's built-in output so we can localize it.
    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return help::render_clap_error(&err, locale),
    };

    // `--version` / `-V`: print the logo and version, then exit.
    if cli.version {
        println!("{}", banner::version_string());
        return ExitCode::SUCCESS;
    }

    // `--help`/`-h`, the `help` subcommand, OR no subcommand at all: show help.
    // (Bare `wxemma` behaves like `wxemma -h` — the friendliest default.)
    let wants_help =
        cli.help || cli.command.is_none() || matches!(cli.command, Some(cli::Commands::Help));
    if wants_help {
        if locale == "zh-CN" {
            println!("{}\n{}", banner::startup_banner(), help::zh_help());
        } else {
            use clap::CommandFactory;
            let _ = cli::Cli::command().print_help();
            println!();
        }
        return ExitCode::SUCCESS;
    }

    let command = cli.command.clone().expect("checked by wants_help above");

    // Completions are handled before building a full context.
    if let cli::Commands::Completions { shell } = &command {
        use clap::CommandFactory;
        let mut cmd = cli::Cli::command();
        clap_complete::generate(*shell, &mut cmd, "wxemma", &mut std::io::stdout());
        return ExitCode::SUCCESS;
    }

    let ops = sysops::RealSystemOps;
    let mut ctx = commands::Ctx {
        ops: &ops,
        cfg,
        apps_dir: PathBuf::from("/Applications"),
        wechat_app: PathBuf::from("/Applications/WeChat.app"),
        json: cli.json,
        yes: cli.yes,
    };

    // Interactive remove: no index and not --yes → list instances and prompt.
    if let cli::Commands::Remove {
        index: None,
        purge_data,
    } = &command
    {
        if !cli.yes {
            let existing = {
                let set = instance::InstanceSet::new(&ops, &ctx.cfg, ctx.apps_dir.clone());
                let existing = set.existing_indices();
                if existing.is_empty() {
                    println!(
                        "{}",
                        output::render(
                            &output::Report::Message("msg.no_instances_remove".into()),
                            cli.json
                        )
                    );
                    return ExitCode::SUCCESS;
                }
                for i in &existing {
                    println!("[{}] {}", i, set.instance_for(*i).display_name);
                }
                existing
            };
            let idx: u8 = dialoguer::Input::new()
                .with_prompt(rust_i18n::t!("prompt.index_to_remove").to_string())
                .interact_text()
                .unwrap_or(0);
            if !existing.contains(&idx) {
                eprintln!(
                    "{}",
                    output::render_error("InstanceNotFound", "invalid index", cli.json)
                );
                return ExitCode::from(2);
            }
            let confirm = dialoguer::Confirm::new()
                .with_prompt(rust_i18n::t!("prompt.confirm_remove", index = idx).to_string())
                .default(false)
                .interact()
                .unwrap_or(false);
            if !confirm {
                println!("{}", rust_i18n::t!("msg.cancelled"));
                return ExitCode::SUCCESS;
            }
            let purge = *purge_data
                || dialoguer::Confirm::new()
                    .with_prompt(rust_i18n::t!("prompt.confirm_purge").to_string())
                    .default(false)
                    .interact()
                    .unwrap_or(false);
            let cmd = cli::Commands::Remove {
                index: Some(idx),
                purge_data: purge,
            };
            return finish(commands::dispatch(&mut ctx, &cmd), cli.json);
        }
    }

    finish(commands::dispatch(&mut ctx, &command), cli.json)
}

fn finish(result: error::Result<output::Report>, json: bool) -> ExitCode {
    match result {
        Ok(report) => {
            println!("{}", output::render(&report, json));
            ExitCode::SUCCESS
        }
        Err(e) => {
            let code = format!("{e:?}");
            let code = code.split_whitespace().next().unwrap_or("Error");
            eprintln!("{}", output::render_error(code, &e.to_string(), json));
            ExitCode::from(e.exit_code())
        }
    }
}
