use std::process::Command;

#[test]
fn test_csv_read() {
    // simply run the app
    let output = Command::new("cargo")
        .arg("run")
        .output()
        .expect("Failed to execute app");

    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("stdout: {}", stdout);
    assert!(stdout.contains("Hello, world!"), "Output did not match");
}
