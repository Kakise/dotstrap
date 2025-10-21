#[cfg(test)]
mod tests {
    use dotstrap::application::run_with_executor;
    use dotstrap::cli::Cli;
    use dotstrap::infrastructure::command::CommandExecutor;
    use std::path::PathBuf;

    struct MockExecutor();

    impl CommandExecutor for MockExecutor {
        fn run(&self, _program: &str, _args: &[&str]) -> dotstrap::Result<()> {
            Ok(())
        }
    }

    fn create_test_cli(
        source: Option<&str>,
        home_dir: Option<std::path::PathBuf>,
        brew: bool,
    ) -> Cli {
        Cli {
            source: "tests/".to_owned() + source.unwrap_or("empty-config"),
            home: home_dir.to_owned(),
            skip_brew: brew,
            dry_run: true,
        }
    }

    #[test]
    fn test_run_with_executor() {
        let executor = MockExecutor();
        let result = run_with_executor(
            create_test_cli(None, Some(PathBuf::from("/home/user")), true),
            &executor,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_executor_brew_enabled() {
        let executor = MockExecutor();
        let result =
            run_with_executor(create_test_cli(Some("config-brew"), None, false), &executor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_executor_no_brew() {
        let executor = MockExecutor();
        let result = run_with_executor(create_test_cli(None, None, false), &executor);
        assert!(result.is_ok());
    }
}
