use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ── Fixtures ──────────────────────────────────────────────────────────────────

const WATER_XYZ: &str =
    "3\nwater\nO  0.000  0.000  0.119\nH  0.000  0.757 -0.477\nH  0.000 -0.757 -0.477\n";

const TEST_CONFIG: &str = r#"
[gedent]
default_extension = "inp"
software = "orca"

[model]
method = "pbe0"
basis_set = "def2-svp"
charge = 0
mult = 1

[resources]
nprocs = 4
mem = 1000
"#;

/// Minimal ORCA SP template used across most tests.
const SP_TEMPLATE: &str = r#"{#
software = "orca"
jobtype = "sp"
requires = ["method", "basis_set", "charge", "mult", "nprocs", "mem", "Molecule"]
description = "Single point energy"
#}
! {{ method }} {{ basis_set }}

%pal
 nprocs {{ nprocs }}
end

%maxcore {{ mem }}

*xyz {{ charge }} {{ mult }}
{{ print_coords(molecule = Molecule) }}
*
"#;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Create an isolated gedent home in a temp dir and return it.
/// Sets up the minimal structure needed for most tests:
///   <tmp>/gedent.toml
///   <tmp>/templates/orca/sp
fn setup_gedent_home() -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    fs::create_dir_all(home.join("templates/orca")).unwrap();
    fs::write(home.join("gedent.toml"), TEST_CONFIG).unwrap();
    fs::write(home.join("templates/orca/sp"), SP_TEMPLATE).unwrap();
    tmp
}

/// Build a `gedent` command with `GEDENT_HOME` pointing at the given path.
fn gedent(gedent_home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("gedent").unwrap();
    cmd.env("GEDENT_HOME", gedent_home);
    cmd
}

// ── gen --print ───────────────────────────────────────────────────────────────

#[test]
fn gen_sp_print_produces_valid_orca_input() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("water.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    gedent(home.path())
        .args(["gen", "sp", "--print"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("! pbe0 def2-svp"))
        .stdout(predicate::str::contains("*xyz 0 1"))
        .stdout(predicate::str::contains("nprocs 4"))
        .stdout(predicate::str::contains("O "));
}

#[test]
fn gen_cli_flag_overrides_config() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("water.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    gedent(home.path())
        .args(["gen", "sp", "--method", "b3lyp", "--print"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("! b3lyp def2-svp"));
}

#[test]
fn gen_var_override_wins_over_config() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("water.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    // --var nprocs=16 should override the config value of 4
    gedent(home.path())
        .args([
            "gen",
            "sp",
            "--show-context",
            "--dry-run",
            "--var",
            "nprocs=16",
        ])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"nprocs\": 16"));
}

// ── gen (file output) ─────────────────────────────────────────────────────────

#[test]
fn gen_writes_output_file_with_correct_content() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("water.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    gedent(home.path())
        .args(["gen", "sp"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .success();

    let output = workdir.path().join("water.inp");
    assert!(output.exists(), "water.inp was not created");
    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("! pbe0 def2-svp"));
    assert!(content.contains("*xyz 0 1"));
}

#[test]
fn gen_multiple_xyz_files_produce_separate_outputs() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz1 = workdir.path().join("mol1.xyz");
    let xyz2 = workdir.path().join("mol2.xyz");
    fs::write(&xyz1, WATER_XYZ).unwrap();
    fs::write(&xyz2, WATER_XYZ).unwrap();

    gedent(home.path())
        .args(["gen", "sp"])
        .arg(&xyz1)
        .arg(&xyz2)
        .current_dir(workdir.path())
        .assert()
        .success();

    assert!(workdir.path().join("mol1.inp").exists());
    assert!(workdir.path().join("mol2.inp").exists());
}

// ── --dry-run ─────────────────────────────────────────────────────────────────

#[test]
fn gen_dry_run_prints_filename_and_writes_nothing() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("water.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    gedent(home.path())
        .args(["gen", "sp", "--dry-run"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("would write water.inp"));

    assert!(
        !workdir.path().join("water.inp").exists(),
        "--dry-run should not write any files"
    );
}

// ── --show-context ────────────────────────────────────────────────────────────

#[test]
fn gen_show_context_outputs_json_with_model_vars() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("water.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    gedent(home.path())
        .args(["gen", "sp", "--show-context", "--dry-run"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"method\": \"pbe0\""))
        .stdout(predicate::str::contains("\"charge\": 0"))
        .stdout(predicate::str::contains("\"nprocs\": 4"));
}

// ── validation ────────────────────────────────────────────────────────────────

#[test]
fn validation_rejects_inconsistent_charge_mult() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("water.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    // Water: 10 electrons, charge=0, mult=2 → (10-1) % 2 ≠ 0 → inconsistent
    gedent(home.path())
        .args(["gen", "sp", "--charge", "0", "--mult", "2"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("inconsistent"));
}

#[test]
fn validation_reports_missing_template_var() {
    let home = setup_gedent_home();
    // Add a template that requires a variable not in config
    fs::create_dir_all(home.path().join("templates/test")).unwrap();
    fs::write(
        home.path().join("templates/test/needsfoo"),
        "{#\nrequires = [\"foo\"]\n#}\n{{ foo }}",
    )
    .unwrap();

    let workdir = tempfile::tempdir().unwrap();
    let xyz = workdir.path().join("mol.xyz");
    fs::write(&xyz, WATER_XYZ).unwrap();

    gedent(home.path())
        .args(["gen", "needsfoo"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("foo"));
}

#[test]
fn validation_warns_on_superposed_atoms() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();
    // Two C atoms at the same position — superposed
    let xyz = workdir.path().join("bad.xyz");
    fs::write(
        &xyz,
        "2\nbad geometry\nC  0.0  0.0  0.0\nC  0.0  0.0  0.0\n",
    )
    .unwrap();

    // Add a template that only needs Molecule (no charge/mult)
    fs::write(
        home.path().join("templates/orca/coords"),
        "{#\nrequires = [\"Molecule\"]\n#}\n{{ print_coords(molecule = Molecule) }}",
    )
    .unwrap();

    gedent(home.path())
        .args(["gen", "coords", "--dry-run"])
        .arg(&xyz)
        .current_dir(workdir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("superposed"));
}

// ── config subcommand ─────────────────────────────────────────────────────────

#[test]
fn config_print_shows_merged_values() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();

    gedent(home.path())
        .args(["config", "print"])
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("pbe0"))
        .stdout(predicate::str::contains("def2-svp"));
}

#[test]
fn config_print_location_shows_chain_and_merged() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();

    gedent(home.path())
        .args(["config", "print", "--location"])
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("gedent.toml"))
        .stdout(predicate::str::contains("merged:"));
}

// ── template subcommand ───────────────────────────────────────────────────────

#[test]
fn template_list_shows_installed_templates() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();

    gedent(home.path())
        .args(["template", "list"])
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("sp"));
}

#[test]
fn template_print_shows_template_source() {
    let home = setup_gedent_home();
    let workdir = tempfile::tempdir().unwrap();

    gedent(home.path())
        .args(["template", "print", "sp"])
        .current_dir(workdir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("print_coords"))
        .stdout(predicate::str::contains("method"));
}
