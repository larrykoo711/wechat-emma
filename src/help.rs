//! Localized `--help` text and clap-error rendering.
//!
//! clap's generated help and errors are compile-time English strings that can't
//! follow the runtime locale. So we disable clap's auto-help and intercept its
//! errors: a hand-written Chinese page / Chinese error when the locale is
//! Chinese, and clap's native English output otherwise.

use std::process::ExitCode;

/// Render a clap parse error (missing arg, unknown command, bad value, or the
/// help/version display errors) in the resolved locale, and return the exit
/// code. For English, defer to clap's own rendering.
pub fn render_clap_error(err: &clap::Error, locale: &str) -> ExitCode {
    use clap::error::ErrorKind;

    // clap models `--help`/`--version` as "errors"; print to stdout, exit 0.
    if matches!(
        err.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        print!("{err}");
        return ExitCode::SUCCESS;
    }

    if locale != "zh-CN" {
        // English: clap's native message is already good.
        eprint!("{err}");
        return ExitCode::from(2);
    }

    let msg = match err.kind() {
        ErrorKind::MissingRequiredArgument => "少了参数。看看 wxemma --help 里的用法。",
        ErrorKind::UnknownArgument => "认不出这个选项。跑 wxemma --help 看看有哪些。",
        ErrorKind::InvalidSubcommand | ErrorKind::NoEquals => {
            "没这个命令。跑 wxemma --help 看看能用哪些命令。"
        }
        ErrorKind::InvalidValue => "这个值不对。跑 wxemma --help 看看该怎么写。",
        _ => "参数有点问题。跑 wxemma --help 看看正确用法。",
    };
    eprintln!("error: {msg}");
    ExitCode::from(2)
}

/// The Chinese help page, shown when the resolved locale is `zh-CN`.
pub fn zh_help() -> &'static str {
    "\
在一台 Mac 上同时开多个微信，各登各的号，互不打架。

用法:
  wxemma <命令> [选项]

命令:
  add          新增一个微信副本（自动挑最小的空编号）
  list         列出所有副本
  status       看看副本和原版微信的版本对不对得上
  remove       删掉某个副本（不给编号就让你挑）
  rebuild      微信升级后，把所有副本按新版本重造一遍
  open         启动全部副本，或按编号启动某一个
  kill         把所有副本进程关掉（原版不动）
  doctor       体检：环境齐不齐、有没有危险副本
  lang         设置界面语言（zh 或 en），记到配置里
  completions  生成 shell 自动补全脚本

选项:
  -V, --version   打印 logo 和版本号
      --json      输出机器可读的 JSON（给脚本和 AI 用）
  -y, --yes       全部默认「是」，不再询问（脚本化必备）
      --lang <值>  临时切语言：zh 或 en
      --verbose   打印更详细的诊断信息
  -h, --help      看这份帮助

数据安全:
  副本各写各的容器，动不到你原版微信的聊天记录。
  删副本前会反复确认；删数据是不可逆的，删了就真没了。

提示:
  add / remove / rebuild 需要 sudo。
  想固定用中文？跑一次 wxemma lang zh，以后就不用每次加 --lang 了。"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zh_help_lists_core_commands() {
        let h = zh_help();
        for cmd in ["add", "list", "remove", "rebuild", "doctor", "lang"] {
            assert!(h.contains(cmd), "help should mention `{cmd}`");
        }
        assert!(h.contains("数据安全"));
    }
}
