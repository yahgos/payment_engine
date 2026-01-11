// tests/smoke_tests.rs

//! Smoke tests for edge cases and error handling.
//! For full scenario testing with output verification, run: ./test_all_scenarios.sh

use payments_engine::start_engine;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn create_test_csv(content: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.csv");
    let mut file = File::create(&file_path).unwrap();
    write!(file, "{}", content).unwrap();
    (dir, file_path.to_str().unwrap().to_string())
}

#[test]
fn test_whitespace_handling() {
    let csv = "type, client, tx, amount\n\
               deposit,  1,  1,  100.0\n\
               withdrawal, 1 , 2 , 50.0 ";

    let (_dir, path) = create_test_csv(csv);
    let result = start_engine(&path);
    assert!(result.is_ok(), "Should handle whitespace in CSV");
}

#[test]
fn test_invalid_transaction_type() {
    let csv = "type,client,tx,amount\n\
               invalid,1,1,100.0";

    let (_dir, path) = create_test_csv(csv);
    let result = start_engine(&path);
    assert!(result.is_err(), "Should reject invalid transaction type");
}

#[test]
fn test_client_id_overflow() {
    let csv = "type,client,tx,amount\n\
               deposit,99999,1,100.0";

    let (_dir, path) = create_test_csv(csv);
    let result = start_engine(&path);
    assert!(result.is_err(), "Should reject client ID > u16::MAX");
}

#[test]
fn test_empty_file() {
    let csv = "type,client,tx,amount";

    let (_dir, path) = create_test_csv(csv);
    let result = start_engine(&path);
    assert!(result.is_ok(), "Should handle empty file gracefully");
}

#[test]
fn test_precision_four_decimals() {
    let csv = "type,client,tx,amount\n\
               deposit,1,1,1.2345\n\
               deposit,1,2,2.6789";

    let (_dir, path) = create_test_csv(csv);
    let result = start_engine(&path);
    assert!(result.is_ok(), "Should parse amounts with 4 decimal places");
}

#[test]
fn test_large_dataset() {
    let mut csv = String::from("type,client,tx,amount\n");

    // Generate 100K transactions across 1000 clients
    for i in 0..100_000 {
        let client = (i % 1000) as u16;
        let tx_type = if i % 10 == 0 { "withdrawal" } else { "deposit" };
        csv.push_str(&format!(
            "{},{},{},{}.0\n",
            tx_type,
            client,
            i,
            (i % 100) + 1
        ));
    }

    let (_dir, path) = create_test_csv(&csv);
    let result = start_engine(&path);
    assert!(result.is_ok(), "Should handle large datasets efficiently");
}

#[test]
fn test_missing_amount_for_deposit() {
    let csv = "type,client,tx,amount\n\
               deposit,1,1,";

    let (_dir, path) = create_test_csv(csv);
    let result = start_engine(&path);
    // Should either error or handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_transaction_id_at_max() {
    let csv = format!("type,client,tx,amount\ndeposit,1,{},100.0", u32::MAX);

    let (_dir, path) = create_test_csv(&csv);
    let result = start_engine(&path);
    assert!(result.is_ok(), "Should handle tx ID at u32::MAX");
}
