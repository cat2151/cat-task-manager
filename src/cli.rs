#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    RunApp,
    Update,
    Check,
    Help,
}

pub const HELP: &str = "\
Usage: cat-task-manager [COMMAND]

Commands:
  update    Self-update cat-task-manager from GitHub
  check     Compare this build with the remote main branch
  help      Print help
";

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let _program = args.next();

    let Some(command) = args.next() else {
        return Ok(Command::RunApp);
    };

    let parsed = match command.as_str() {
        "update" => Command::Update,
        "check" => Command::Check,
        "help" | "-h" | "--help" => Command::Help,
        unknown => return Err(format!("unknown command: {unknown}")),
    };

    if let Some(extra) = args.next() {
        return Err(format!("{command} does not accept argument: {extra}"));
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn no_args_runs_app() {
        assert_eq!(parse(args(&["cat-task-manager"])), Ok(Command::RunApp));
    }

    #[test]
    fn parses_update_command() {
        assert_eq!(
            parse(args(&["cat-task-manager", "update"])),
            Ok(Command::Update)
        );
    }

    #[test]
    fn parses_check_command() {
        assert_eq!(
            parse(args(&["cat-task-manager", "check"])),
            Ok(Command::Check)
        );
    }

    #[test]
    fn parses_help_command() {
        assert_eq!(
            parse(args(&["cat-task-manager", "--help"])),
            Ok(Command::Help)
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(
            parse(args(&["cat-task-manager", "unknown"])),
            Err("unknown command: unknown".to_string())
        );
    }

    #[test]
    fn rejects_extra_argument() {
        assert_eq!(
            parse(args(&["cat-task-manager", "check", "extra"])),
            Err("check does not accept argument: extra".to_string())
        );
    }
}
