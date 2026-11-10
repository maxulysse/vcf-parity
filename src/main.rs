mod input;
mod pair;

use std::collections::HashMap;
use std::io::{self, BufWriter, StdoutLock, Write};
use std::process::ExitCode;

use noodles::vcf;

use input::{ContigOrder, Input, Record, Scope, open};
use pair::{Pair, pair_by};

const USAGE: &str = "usage: vcf-parity <a.vcf[.gz]> <b.vcf[.gz]>";
const COLUMNS: &str = "chrom\tpos\tref\talt\tsample\tfield\tkind\tvalue_a\tvalue_b";
const MISSING: &str = ".";

#[derive(Default)]
struct Counts {
    compared: usize,
    only_a: usize,
    only_b: usize,
    samples: usize,
    rows: usize,
}

struct Report {
    out: BufWriter<StdoutLock<'static>>,
    shared_samples: Vec<String>,
    counts: Counts,
}

impl Report {
    /// Writes one discrepancy row; `at` is `None` for header-level rows.
    fn emit(
        &mut self,
        at: Option<&Record>,
        sample: &str,
        field: &str,
        values: Pair<&str>,
    ) -> io::Result<()> {
        self.counts.rows += 1;
        let (kind, a, b) = match values {
            Pair::Both(a, b) => ("differs", a, b),
            Pair::OnlyA(a) => ("missing_in_b", a, MISSING),
            Pair::OnlyB(b) => ("missing_in_a", MISSING, b),
        };
        match at {
            Some(r) => write!(
                self.out,
                "{}\t{}\t{}\t{}",
                r.chrom, r.pos, r.reference, r.alternate
            )?,
            None => write!(self.out, ".\t.\t.\t.")?,
        }
        writeln!(self.out, "\t{sample}\t{field}\t{kind}\t{a}\t{b}")
    }

    fn samples(&mut self, a: &[String], b: &[String]) -> io::Result<()> {
        for pair in pair_by(a, b, |x, y| x == y) {
            if let Pair::Both(..) = pair {
                continue;
            }
            self.counts.samples += 1;
            let name = pair.either().as_str();
            self.emit(None, name, "sample", pair.map(String::as_str))?;
        }
        Ok(())
    }

    /// Compares the records of a and b at one position, paired by REF and ALT.
    fn records(&mut self, a: &[Record], b: &[Record]) -> io::Result<()> {
        for pair in pair_by(a, b, Record::same_alleles) {
            let record = match pair {
                Pair::Both(x, y) => {
                    self.fields(x, y)?;
                    continue;
                }
                Pair::OnlyA(x) => {
                    self.counts.only_a += 1;
                    x
                }
                Pair::OnlyB(y) => {
                    self.counts.only_b += 1;
                    y
                }
            };
            self.emit(Some(record), MISSING, "record", pair.map(|_| MISSING))?;
        }
        Ok(())
    }

    fn fields(&mut self, a: &Record, b: &Record) -> io::Result<()> {
        self.counts.compared += 1;
        for pair in pair_by(&a.fields, &b.fields, |x, y| x.0 == y.0) {
            if let Pair::Both(x, y) = &pair
                && x.1 == y.1
            {
                continue;
            }
            let (scope, field) = &pair.either().0;
            let sample = match *scope {
                Scope::Site => MISSING.to_string(),
                Scope::Sample(i) => self.shared_samples[i].clone(),
            };
            self.emit(Some(a), &sample, field, pair.map(|f| f.1.as_str()))?;
        }
        Ok(())
    }
}

fn run(path_a: &str, path_b: &str) -> io::Result<bool> {
    let (reader_a, header_a) = open(path_a)?;
    let (reader_b, header_b) = open(path_b)?;

    let mut order = ContigOrder(HashMap::new());
    for name in header_a.contigs().keys().chain(header_b.contigs().keys()) {
        order.index(name);
    }

    let names_a: Vec<String> = header_a.sample_names().iter().cloned().collect();
    let names_b: Vec<String> = header_b.sample_names().iter().cloned().collect();
    let shared_samples: Vec<String> = names_a
        .iter()
        .filter(|n| names_b.contains(n))
        .cloned()
        .collect();
    let columns = |names: &[String]| -> Vec<usize> {
        shared_samples
            .iter()
            .map(|s| names.iter().position(|n| n == s).unwrap())
            .collect()
    };
    let input = |path: &str, reader, names: &[String]| Input {
        path: path.to_string(),
        reader,
        buf: vcf::Record::default(),
        sample_columns: columns(names),
        next: None,
        last_key: (0, 0),
    };
    let mut a = input(path_a, reader_a, &names_a);
    let mut b = input(path_b, reader_b, &names_b);

    let mut report = Report {
        out: BufWriter::new(io::stdout().lock()),
        shared_samples,
        counts: Counts::default(),
    };
    writeln!(report.out, "{COLUMNS}")?;
    report.samples(&names_a, &names_b)?;

    let mut block_a = a.block(&mut order)?;
    let mut block_b = b.block(&mut order)?;
    while let Some(key) = block_a
        .first()
        .into_iter()
        .chain(block_b.first())
        .map(Record::key)
        .min()
    {
        let take = |block: &mut Vec<Record>| {
            if block.first().map(Record::key) == Some(key) {
                std::mem::take(block)
            } else {
                Vec::new()
            }
        };
        let (at_a, at_b) = (take(&mut block_a), take(&mut block_b));
        report.records(&at_a, &at_b)?;
        if block_a.is_empty() {
            block_a = a.block(&mut order)?;
        }
        if block_b.is_empty() {
            block_b = b.block(&mut order)?;
        }
    }
    report.out.flush()?;

    let c = &report.counts;
    eprintln!(
        "vcf-parity: {} records compared, {} only in a, {} only in b, {} field differences, {} samples in one file only",
        c.compared,
        c.only_a,
        c.only_b,
        c.rows - c.only_a - c.only_b - c.samples,
        c.samples
    );
    Ok(c.rows > 0)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(a, b) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(1),
        Err(e) => {
            eprintln!("vcf-parity: {e}");
            ExitCode::from(2)
        }
    }
}
