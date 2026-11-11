use std::process::{Command, Output};

fn vcf_parity(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vcf-parity"))
        .args(args)
        .output()
        .unwrap()
}

fn run(a: &str, b: &str) -> Output {
    vcf_parity(&[&format!("tests/data/{a}"), &format!("tests/data/{b}")])
}

#[test]
fn reports_every_kind_of_discrepancy() {
    let out = run("a.vcf", "b.vcf");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "chrom\tpos\tref\talt\tsample\tfield\tkind\tvalue_a\tvalue_b\n\
         .\t.\t.\t.\tS2\tsample\tmissing_in_a\t.\tS2\n\
         chr20\t2000\tG\t<INS>\t.\tQUAL\tdiffers\t30\t31\n\
         chr20\t2000\tG\t<INS>\t.\tINFO/IMPRECISE\tmissing_in_b\ttrue\t.\n\
         chr20\t2000\tG\t<INS>\tS1\tFORMAT/GT\tdiffers\t1/1\t0/1\n\
         chr20\t3000\tC\t<DEL>\t.\trecord\tmissing_in_b\t.\t.\n\
         chr20\t4000\tA\t<DEL>\t.\trecord\tmissing_in_a\t.\t.\n"
    );
}

#[test]
fn identical_files_exit_zero() {
    for file in [
        "a.vcf",
        "nf-core/NA12878_GIAB.chr22.vcf.gz",
        "nf-core/NA24385_sv.vcf.gz",
        "nf-core/test.rnaseq.vcf.gz",
    ] {
        let out = run(file, file);
        assert_eq!(out.status.code(), Some(0), "{file}");
        assert_eq!(
            out.stdout.iter().filter(|&&c| c == b'\n').count(),
            1,
            "{file}"
        );
    }
}

#[test]
fn unsorted_input_is_an_error() {
    let out = run("a.vcf", "unsorted.vcf");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("not sorted")
    );
}

#[test]
fn missing_sample_column_is_an_error() {
    let out = run("b.vcf", "missing_sample.vcf");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("fewer sample columns")
    );
}

#[test]
fn version_and_help_exit_zero() {
    for flag in ["--version", "-V", "--help", "-h"] {
        let out = vcf_parity(&[flag]);
        assert_eq!(out.status.code(), Some(0), "{flag}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            stdout.starts_with(&format!("vcf-parity {}", env!("CARGO_PKG_VERSION"))),
            "{flag}"
        );
    }
}

#[test]
fn wrong_arguments_exit_two() {
    assert_eq!(vcf_parity(&["only-one.vcf"]).status.code(), Some(2));
}

fn run_with(a: &str, b: &str, tolerances: &[&str]) -> Output {
    let mut args = vec![format!("tests/data/{a}"), format!("tests/data/{b}")];
    for t in tolerances {
        args.push("--tolerance".to_string());
        args.push(t.to_string());
    }
    vcf_parity(&args.iter().map(String::as_str).collect::<Vec<_>>())
}

#[test]
fn tolerances_hide_close_values() {
    let out = run_with(
        "numeric_a.vcf",
        "numeric_b.vcf",
        &[
            "QUAL=abs:0.5",
            "INFO/AF=rel:1e-6",
            "FORMAT/GQ=abs:1",
            "FORMAT/AD=abs:1",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "chrom\tpos\tref\talt\tsample\tfield\tkind\tvalue_a\tvalue_b\n\
         chr1\t200\tG\tT\tS1\tFORMAT/GQ\tdiffers\t.\t30\n\
         chr1\t200\tG\tT\tS1\tFORMAT/AD\tdiffers\t5,5\t5,5,5\n"
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("2 field differences, 5 field differences within tolerance")
    );
}

#[test]
fn values_within_tolerance_exit_zero() {
    let out = run_with("numeric_a.vcf", "numeric_c.vcf", &[]);
    assert_eq!(out.status.code(), Some(1));
    let out = run_with("numeric_a.vcf", "numeric_c.vcf", &["QUAL=abs:1"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn missing_values_and_length_mismatches_still_differ() {
    let out = run_with(
        "numeric_a.vcf",
        "numeric_b.vcf",
        &[
            "QUAL=abs:1",
            "INFO/AF=abs:0.001",
            "FORMAT/GQ=abs:100",
            "FORMAT/AD=abs:1",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn exact_tolerance_compares_numbers() {
    let out = run_with("numeric_a.vcf", "numeric_b.vcf", &["INFO/AF=exact"]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("chr1\t100\tA\tC\t.\tINFO/AF"));
    assert!(stdout.contains("chr1\t200\tG\tT\t.\tINFO/AF\tdiffers\t0.25\t0.2500001"));
}

#[test]
fn invalid_tolerances_exit_two() {
    for tolerance in [
        "QUAL",
        "QUAL=near:1",
        "ID=abs:1",
        "FORMAT/GT=abs:1",
        "INFO/SOURCE=abs:1",
    ] {
        let out = run_with("numeric_a.vcf", "numeric_b.vcf", &[tolerance]);
        assert_eq!(out.status.code(), Some(2), "{tolerance}");
    }
    let out = run_with(
        "numeric_a.vcf",
        "numeric_b.vcf",
        &["QUAL=abs:1", "QUAL=abs:2"],
    );
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn undeclared_tolerance_field_warns() {
    let out = run_with("numeric_a.vcf", "numeric_b.vcf", &["INFO/NOPE=abs:1"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("INFO/NOPE is not declared in either header")
    );
}
