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

fn write_transparent_icon(path: &Path) {
    let mut image = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let pixel = if x > 6 && x < 26 && y > 6 && y < 26 {
                Rgba([10, 20, 30, 255])
            } else {
                Rgba([0, 0, 0, 0])
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

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args(["analyze", input.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(contains("\"image_type\": \"logo\""));

    Command::cargo_bin("edgesvg")
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

    Command::cargo_bin("edgesvg")
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

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args(["render", svg.to_str().unwrap(), png.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("wrote"));
    assert!(png.exists());

    Command::cargo_bin("edgesvg")
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

#[test]
fn logo_premium_and_auto_commands_work_from_cli() {
    let dir = tempdir().unwrap();
    let logo_input = dir.path().join("logo.png");
    let icon_input = dir.path().join("icon.png");
    let logo_svg = dir.path().join("logo.svg");
    let premium_svg = dir.path().join("premium.svg");
    let auto_svg = dir.path().join("auto.svg");
    write_fixture(&logo_input);
    write_transparent_icon(&icon_input);

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args([
            "logo",
            logo_input.to_str().unwrap(),
            logo_svg.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"metrics\""));

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args([
            "premium",
            logo_input.to_str().unwrap(),
            premium_svg.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"metrics\""));

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args([
            "auto",
            icon_input.to_str().unwrap(),
            auto_svg.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"decision\""));

    assert!(logo_svg.exists());
    assert!(premium_svg.exists());
    assert!(auto_svg.exists());
}

#[test]
fn info_optimize_and_method_fallbacks_work_from_cli() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("fixture.png");
    let svg = dir.path().join("fixture.svg");
    let smart_svg = dir.path().join("smart.svg");
    write_fixture(&input);
    std::fs::write(
        &svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path fill="#ff0000" opacity="1" d="M 0.1234 0.5678 L 9.8765 9.4321"/></svg>"##,
    )
    .unwrap();

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args(["info", input.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .stdout(contains("\"recommended\""));

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args([
            "optimize",
            svg.to_str().unwrap(),
            "--json",
            "--precision",
            "1",
        ])
        .assert()
        .success()
        .stdout(contains("\"reduction_percent\""));

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args([
            "convert",
            input.to_str().unwrap(),
            dir.path().join("bayesian.svg").to_str().unwrap(),
            "--method",
            "bayesian",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"requested_method\": \"bayesian\""));

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args([
            "smart",
            input.to_str().unwrap(),
            smart_svg.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"metrics\""));

    Command::cargo_bin("edgesvg")
        .unwrap()
        .args([
            "convert",
            input.to_str().unwrap(),
            dir.path().join("optimal.svg").to_str().unwrap(),
            "--method",
            "optimal",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"requested_method\": \"optimal\""));
}
