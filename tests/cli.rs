use std::process::{Command, Output};

fn run(a: &str, b: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vcf-parity"))
        .args([format!("tests/data/{a}"), format!("tests/data/{b}")])
        .output()
        .unwrap()
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
        let out = Command::new(env!("CARGO_BIN_EXE_vcf-parity"))
            .arg(flag)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{flag}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            stdout.starts_with(&format!("vcf-parity {}", env!("CARGO_PKG_VERSION"))),
            "{flag}"
        );
    }
}
