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

    let expected_output = "\
                StringRecord([\"deposit\", \"1\", \"1\", \"1.0\"])\n\
                StringRecord([\"deposit\", \"2\", \"2\", \"2.0\"])\n\
                StringRecord([\"deposit\", \"1\", \"3\", \"2.0\"])\n\
                StringRecord([\"withdrawal\", \"1\", \"4\", \"1.5\"])\n\
                StringRecord([\"withdrawal\", \"2\", \"5\", \"3.0\"])\n";

    assert_eq!(
        stdout, expected_output,
        "The output does not match the expected format"
    );

    // println!("stdout: {}\n", stdout);

    // remove file
    let _ = std::fs::remove_file(csv_path);
}
