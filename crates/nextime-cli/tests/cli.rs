// SPDX-License-Identifier: AGPL-3.0-only
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn command_line_and_errors() {
    let result = Command::new(env!("CARGO_BIN_EXE_nextime"))
        .arg("hashi")
        .output()
        .unwrap();
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("reading: はし"));
    assert!(output.contains("1. 橋"));
    assert!(output.contains("箸"));
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_nextime"))
            .args(["--limit", "0"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_nextime"))
            .arg("--unknown")
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn stdin_supports_multiple_lines() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nextime"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"nihongo\nhashi\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("日本語"));
    assert!(text.contains("橋"));
}

#[test]
fn session_demo_selects_only_on_enter() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nextime"))
        .arg("session")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"text h\ntext a\ntext s\ntext h\ntext i\nspace\nnext\nenter\nquit\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("preedit: はし"));
    assert!(text.contains("> 2. 箸"));
    assert_eq!(text.matches("commit:").count(), 1);
    assert!(text.contains("commit: 箸"));
}

#[test]
fn dictionary_cli_build_verify_convert_register_and_unpack() {
    use std::fs;
    let directory =
        std::env::temp_dir().join(format!("nextime-cli-nud-test-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            for item in std::fs::read_dir(&self.0).unwrap() {
                let item = item.unwrap();
                assert!(item.file_type().unwrap().is_file());
                std::fs::remove_file(item.path()).unwrap();
            }
            std::fs::remove_dir(&self.0).unwrap();
        }
    }
    let cleanup = Cleanup(directory);
    let base = cleanup.0.join("base.nud");
    let journal = cleanup.0.join("words.nxl");
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dictionaries/sample-ja.tsv");
    let binary = env!("CARGO_BIN_EXE_nextime");
    let build = Command::new(binary)
        .args(["dict", "build"])
        .arg(&source)
        .arg(&base)
        .arg("42")
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(
        !Command::new(binary)
            .args(["dict", "build"])
            .arg(&source)
            .arg(&base)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        Command::new(binary)
            .args(["dict", "verify"])
            .arg(&base)
            .output()
            .unwrap()
            .status
            .success()
    );
    let output = Command::new(binary)
        .arg("--nud")
        .arg(&base)
        .arg("hashi")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("1. 橋"));
    let add = Command::new(binary)
        .args(["dict", "add"])
        .arg(&base)
        .arg(&journal)
        .args(["ねくすと", "NextIME", "noun", "5000"])
        .output()
        .unwrap();
    assert!(add.status.success());
    let output = Command::new(binary)
        .arg("--nud")
        .arg(&base)
        .arg("--journal")
        .arg(&journal)
        .arg("nekusuto")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("1. NextIME")
    );
    let unpack = Command::new(binary)
        .args(["dict", "unpack"])
        .arg(&base)
        .output()
        .unwrap();
    assert!(unpack.status.success());
    assert!(
        String::from_utf8(unpack.stdout)
            .unwrap()
            .contains("はし\t橋\tnoun\t8000")
    );
    assert!(
        !Command::new(binary)
            .arg("--journal")
            .arg(&journal)
            .arg("hashi")
            .output()
            .unwrap()
            .status
            .success()
    );
}
