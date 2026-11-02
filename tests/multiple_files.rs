use std::process::Command;

#[test]
fn multiple_file_arguments_are_rejected_before_reading_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_logsift"))
        .args(["first.log", "second.log"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("one input file"));
}
