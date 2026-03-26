use std::path::Path;

use assert_cmd::Command;
use image::{Rgba, RgbaImage};
use predicates::str::contains;
use tempfile::tempdir;

fn write_fixture(path: &Path) {
    let mut image = RgbaImage::new(80, 40);
    for y in 0..40 {
        for x in 0..80 {
            let pixel = if x < 26 {
                Rgba([255, 255, 255, 255])
            } else if x < 53 {
                Rgba([180, 0, 0, 255])
            } else {
                Rgba([0, 0, 180, 255])
            };
            image.put_pixel(x, y, pixel);
        }
    }
    image.save(path).unwrap();
}

#[test]
fn analyze_convert_compare_render_and_benchmark_work_from_cli() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().join("inputs");
    let output_dir = dir.path().join("bench");
    std::fs::create_dir_all(&input_dir).unwrap();
    let input = input_dir.join("fixture.png");
    let svg = dir.path().join("fixture.svg");
    let png = dir.path().join("fixture.rendered.png");
    let report_json = dir.path().join("report.json");
    let report_md = dir.path().join("report.md");
    write_fixture(&input);

    Command::cargo_bin("vectalab")
        .unwrap()
        .args(["analyze", input.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(contains("\"image_type\": \"logo\""));

    Command::cargo_bin("vectalab")
        .unwrap()
        .args([
            "convert",
            input.to_str().unwrap(),
            svg.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"metrics\""));

    Command::cargo_bin("vectalab")
        .unwrap()
        .args([
            "compare",
            input.to_str().unwrap(),
            svg.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"ssim\""));

    Command::cargo_bin("vectalab")
        .unwrap()
        .args(["render", svg.to_str().unwrap(), png.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("wrote"));
    assert!(png.exists());

    Command::cargo_bin("vectalab")
        .unwrap()
        .args([
            "benchmark",
            "--input-dir",
            input_dir.to_str().unwrap(),
            "--output-dir",
            output_dir.to_str().unwrap(),
            "--json-path",
            report_json.to_str().unwrap(),
            "--markdown-path",
            report_md.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("\"entries\""));

    assert!(report_json.exists());
    assert!(report_md.exists());
}
