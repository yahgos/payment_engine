use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_all_input_scenarios() {
    let inputs_dir = PathBuf::from("tests/inputs");
    let expected_dir = PathBuf::from("tests/expected_outputs");
    let actual_dir = PathBuf::from("tests/actual_outputs");

    // Clean and create output directory
    let _ = fs::remove_dir_all(&actual_dir);
    fs::create_dir_all(&actual_dir).expect("Failed to create actual_outputs");

    // Build first
    println!("Building project...");
    let build = Command::new("cargo")
        .args(&["build", "--release", "--quiet"])
        .status()
        .expect("Failed to build");
    assert!(build.success(), "Build failed");

    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    // Process each input file
    for entry in fs::read_dir(&inputs_dir).expect("Can't read inputs dir") {
        let entry = entry.expect("Invalid entry");
        let input_path = entry.path();

        if input_path.extension().map_or(true, |ext| ext != "csv") {
            continue;
        }

        let filename = input_path.file_name().unwrap().to_str().unwrap();
        let base_name = input_path.file_stem().unwrap().to_str().unwrap();

        let expected_path = expected_dir.join(format!("{}_output.csv", base_name));
        let actual_path = actual_dir.join(format!("{}_output.csv", base_name));

        println!("\nTesting: {}", filename);

        if !expected_path.exists() {
            println!("⚠ SKIP - No expected output");
            continue;
        }

        // Run engine
        let output = Command::new("cargo")
            .args(&[
                "run",
                "--release",
                "--quiet",
                "--",
                input_path.to_str().unwrap(),
            ])
            .output()
            .expect("Failed to run engine");

        if !output.status.success() {
            println!("✗ FAIL - Engine crashed");
            failed += 1;
            failures.push(filename.to_string());
            continue;
        }

        // Write actual output
        fs::write(&actual_path, &output.stdout).expect("Failed to write output");

        // Compare
        let expected = normalize_csv(&fs::read_to_string(&expected_path).unwrap());
        let actual = normalize_csv(&String::from_utf8_lossy(&output.stdout));

        if expected == actual {
            println!("✓ PASS");
            passed += 1;
        } else {
            println!("✗ FAIL - Output differs");
            println!("  Expected: {:?}", expected_path);
            println!("  Actual:   {:?}", actual_path);
            failed += 1;
            failures.push(filename.to_string());
        }
    }

    // Print summary
    println!("\n==================");
    println!("Scenario Test Summary");
    println!("==================");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if !failures.is_empty() {
        println!("\nFailed scenarios:");
        for failure in &failures {
            println!("  - {}", failure);
        }
    }

    assert_eq!(failed, 0, "{} scenario test(s) failed", failed);
}

fn normalize_csv(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return String::new();
    }

    let header = lines[0].trim();
    let mut data_lines: Vec<&str> = lines[1..].iter().map(|s| s.trim()).collect();

    data_lines.sort_by_key(|line| {
        line.split(',')
            .next()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0)
    });

    let mut result = String::from(header);
    result.push('\n');
    for line in data_lines {
        if !line.is_empty() {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}
