mod input;
mod pair;

use std::io::{self, BufWriter, StdoutLock, Write};
use std::process::ExitCode;

use input::{Inputs, Record, Scope, open_pair};
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
    fields: usize,
}

impl Counts {
    fn any(&self) -> bool {
        self.only_a + self.only_b + self.samples + self.fields > 0
    }
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
        for pair in pair_by(&a.fields, &b.fields, |x, y| {
            x.scope == y.scope && x.name == y.name
        }) {
            if let Pair::Both(x, y) = &pair
                && x.value == y.value
            {
                continue;
            }
            self.counts.fields += 1;
            let field = pair.either();
            let sample = match field.scope {
                Scope::Site => MISSING.to_string(),
                Scope::Sample(i) => self.shared_samples[i].clone(),
            };
            self.emit(
                Some(a),
                &sample,
                &field.name,
                pair.map(|f| f.value.as_str()),
            )?;
        }
        Ok(())
    }
}

fn run(path_a: &str, path_b: &str) -> io::Result<bool> {
    let Inputs {
        mut a,
        mut b,
        mut order,
        names_a,
        names_b,
        shared_samples,
    } = open_pair(path_a, path_b)?;

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
        c.compared, c.only_a, c.only_b, c.fields, c.samples
    );
    Ok(c.any())
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
