use std::{
    fs::{
        create_dir_all,
        File,
    },
    io::{
        BufWriter,
        Write,
    },
    process::Command,
};

use crate::helpers::{
    generate_csv_input,
    generate_transactions,
};

#[test]
fn test_e2e_mixed_client_mixed_tx_types() {
    /* Arrange */
    // Generate sample transactions
    let transactions = generate_transactions(32);

    // Generate the CSV with a specific filename
    let input_file_path = "input_1.csv";
    let result = generate_csv_input(input_file_path, transactions);

    // Assert sample input exists
    assert!(result.is_ok());
    let file_path = result.unwrap();

    /* Act */
    let output = Command::new("cargo")
        .arg("run")
        .arg(&file_path)
        .output()
        .expect("Failed to execute app");

    // Define the directory and file path for the output file
    let output_dir = "test_output";
    let output_file_path = format!("{}/output_1.txt", output_dir);

    // Create the directory if it doesn't exist
    create_dir_all(output_dir).expect("Failed to create output directory");

    // Create the output file and write the captured output
    let output_file = File::create(&output_file_path).expect("Failed to create output file");

    let mut writer = BufWriter::new(output_file);

    // Write stdout to the output file
    writer
        .write_all(&output.stdout)
        .expect("Failed to write stdout to file");

    // Write stderr to the output file (if needed)
    writer
        .write_all(&output.stderr)
        .expect("Failed to write stderr to file");

    // Ensure output is flushed
    writer
        .flush()
        .expect("Failed to flush output file");

    // NOte: assertion was done by hand comparing output.

    //remove_file(input_file_path).expect("Failed to remove test CSV file");
}
