use std::process::{Command, Stdio};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_rs-gomoku")
}

#[test]
fn root_and_replay_help_separate_normal_only_options() {
    let root = Command::new(binary()).arg("--help").output().unwrap();
    assert!(root.status.success());
    let root = String::from_utf8(root.stdout).unwrap();
    for option in ["--size", "--rule", "--record-dir", "--lang"] {
        assert!(root.contains(option), "{root}");
    }

    let replay = Command::new(binary()).args(["replay", "--help"]).output().unwrap();
    assert!(replay.status.success());
    let replay = String::from_utf8(replay.stdout).unwrap();
    for option in ["--interval", "--paused", "--exit-on-end", "--print", "--lang"] {
        assert!(replay.contains(option), "{replay}");
    }
    for normal_only in ["--size", "--rule", "--record-dir"] {
        assert!(!replay.contains(normal_only), "{normal_only} leaked into replay help:\n{replay}");
    }
}

#[test]
fn replay_rejects_normal_only_options_even_before_the_subcommand() {
    for normal_only in [["--size", "9"], ["--rule", "standard"], ["--record-dir", "."]] {
        let output = Command::new(binary())
            .args(normal_only)
            .args(["replay", "examples/records/unfinished.gmkrec", "--print"])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{normal_only:?} was silently accepted");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("cannot be used with replay"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn replay_print_lists_moves_once_then_final_board_and_result() {
    let output = Command::new(binary())
        .args(["replay", "examples/records/black-wins-15.gmkrec", "--print"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Move 1: Alice Black f8"), "{text}");
    assert!(text.contains("Move 15: Alice Black i10"), "{text}");
    assert!(text.contains("Result: Alice wins"), "{text}");
    assert_eq!(text.lines().filter(|line| line.len() == 29 && line.contains('.')).count(), 15);
}

#[test]
fn replay_stdin_and_error_prefix_follow_cli_contract() {
    let mut child = Command::new(binary())
        .args(["replay", "-", "--print"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write as _;
    child.stdin.as_mut().unwrap().write_all(b"size = 9\na1\nb1\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("Result: unfinished"));

    let output = Command::new(binary()).args(["--size", "6"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("rs-gomoku:"));
}

#[test]
fn replay_print_uses_requested_japanese_language() {
    let output = Command::new(binary())
        .args(["replay", "examples/records/black-wins-15.gmkrec", "--lang", "ja", "--print"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("手数 15: Alice 黒 i10"), "{text}");
    assert!(text.contains("結果: Alice の勝ち"), "{text}");
}
