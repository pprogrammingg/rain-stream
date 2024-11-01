use crate::helpers;
use std::fs::File;
use std::process::Command;

#[test]
fn test_csv_read() {
    // arrange
    let csv_path = String::from("input_1.csv");
    helpers::write_csv(&csv_path).expect("failed to read CSV");

    // Ensure the file was created and can be read
    assert!(
        File::open(&csv_path).is_ok(),
        "CSV file should be created and accessible."
    );

    let output = Command::new("cargo")
        .arg("run")
        .arg(&csv_path)
        .output()
        .expect("Failed to execute app");

    // output
    let stdout = String::from_utf8_lossy(&output.stdout);

    // // error
    // let stderr = String::from_utf8_lossy(&output.stderr);
    //
    println!("stdout: {}", stdout);
    // println!("stderr: {}", stderr);
    //
    // if !stderr.is_empty() {
    //     eprintln!("Error output: {}", stderr);
    // }
    //
    // assert!(
    //     stdout.contains("type,client,tx,amount"),
    //     "Header not found in output"
    // );
    // assert!(
    //     stdout.contains("deposit,1,1,1.0"),
    //     "Expected deposit record not found in output"
    // );
    // assert!(
    //     stdout.contains("deposit,2,2,2.0"),
    //     "Expected deposit record not found in output"
    // );
    // assert!(
    //     stdout.contains("deposit,1,3,2.0"),
    //     "Expected deposit record not found in output"
    // );
    // assert!(
    //     stdout.contains("withdrawal,1,4,1.5"),
    //     "Expected withdrawal record not found in output"
    // );
    // assert!(
    //     stdout.contains("withdrawal,2,5,3.0"),
    //     "Expected withdrawal record not found in output"
    // );

    // remove file
    let _ = std::fs::remove_file(csv_path);
}
