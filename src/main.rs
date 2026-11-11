mod header;
mod ignore;
mod input;
mod pair;
mod tolerance;

use std::ffi::OsString;
use std::io::{self, BufWriter, StdoutLock, Write};
use std::process::ExitCode;

use header::Difference;
use ignore::Patterns;
use input::{Inputs, Record, Scope, open_pair};
use pair::{Pair, pair_by};
use tolerance::{Tolerance, Tolerances};

#[derive(clap::Parser)]
#[command(
    version,
    about,
    help_template = "{name} {version}\n{about}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}",
    after_help = "Arguments can be read from a file, one per line: @FILE.\nWrites one TSV row per discrepancy to stdout and a summary to stderr.\nExit code: 0 files match, 1 files differ, 2 error."
)]
struct Cli {
    /// First VCF or bgzipped VCF
    a: String,
    /// Second VCF or bgzipped VCF
    b: String,
    /// Tolerance for a numeric field, repeatable: FIELD=exact, FIELD=abs:X, FIELD=rel:X or FIELD=abs:X,rel:Y
    #[arg(long = "tolerance", value_name = "FIELD=SPEC", value_parser = tolerance::parse)]
    tolerances: Vec<(String, Tolerance)>,
    /// Skip fields matching PATTERN, repeatable: /-separated segments, * matches one segment, a pattern also matches every field below it
    #[arg(long = "ignore", value_name = "PATTERN")]
    ignore: Vec<String>,
}

const COLUMNS: &str = "chrom\tpos\tref\talt\tsample\tfield\tkind\tvalue_a\tvalue_b";
const MISSING: &str = ".";

#[derive(Default)]
struct Counts {
    compared: usize,
    only_a: usize,
    only_b: usize,
    samples: usize,
    header: usize,
    fields: usize,
    tolerated: usize,
}

impl Counts {
    fn any(&self) -> bool {
        self.only_a + self.only_b + self.samples + self.header + self.fields > 0
    }
}

struct Report {
    out: BufWriter<StdoutLock<'static>>,
    shared_samples: Vec<String>,
    tolerances: Tolerances,
    ignore: Patterns,
    counts: Counts,
}

/// What a discrepancy row is about, which decides the summary count it adds to.
enum Row {
    Header,
    Sample,
    Record,
    Field,
}

impl Report {
    /// Counts and writes one discrepancy row, unless `--ignore` matches its field; `at` is `None` for header and sample rows.
    fn emit(
        &mut self,
        row: Row,
        at: Option<&Record>,
        sample: &str,
        field: &str,
        values: Pair<&str>,
    ) -> io::Result<()> {
        if self.ignore.matches(field) {
            return Ok(());
        }
        let c = &mut self.counts;
        *match (row, &values) {
            (Row::Header, _) => &mut c.header,
            (Row::Sample, _) => &mut c.samples,
            (Row::Field, _) => &mut c.fields,
            (Row::Record, Pair::OnlyB(_)) => &mut c.only_b,
            (Row::Record, _) => &mut c.only_a,
        } += 1;
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

    fn header(&mut self, differences: &[Difference]) -> io::Result<()> {
        for Difference { field, values } in differences {
            self.emit(
                Row::Header,
                None,
                MISSING,
                field,
                values.as_ref().map(String::as_str),
            )?;
        }
        Ok(())
    }

    fn samples(&mut self, a: &[String], b: &[String]) -> io::Result<()> {
        for pair in pair_by(a, b, |x, y| x == y) {
            if let Pair::Both(..) = pair {
                continue;
            }
            let name = pair.either().as_str();
            self.emit(Row::Sample, None, name, "sample", pair.map(String::as_str))?;
        }
        Ok(())
    }

    /// Compares the records of a and b at one position, paired by REF and ALT.
    fn records(&mut self, a: &[Record], b: &[Record]) -> io::Result<()> {
        for pair in pair_by(a, b, Record::same_alleles) {
            if let Pair::Both(x, y) = pair {
                self.fields(x, y)?;
                continue;
            }
            let record = *pair.either();
            self.emit(
                Row::Record,
                Some(record),
                MISSING,
                "record",
                pair.map(|_| MISSING),
            )?;
        }
        Ok(())
    }

    fn fields(&mut self, a: &Record, b: &Record) -> io::Result<()> {
        self.counts.compared += 1;
        for pair in pair_by(&a.fields, &b.fields, |x, y| {
            x.scope == y.scope && x.name == y.name
        }) {
            if let Pair::Both(x, y) = &pair {
                if x.value == y.value {
                    continue;
                }
                // Checked before the tolerance, so ignored fields are not counted as tolerated.
                if !self.ignore.matches(&x.name)
                    && self.tolerances.accepts(&x.name, &x.value, &y.value)
                {
                    self.counts.tolerated += 1;
                    continue;
                }
            }
            let field = pair.either();
            let sample = match field.scope {
                Scope::Site => MISSING.to_string(),
                Scope::Sample(i) => self.shared_samples[i].clone(),
            };
            self.emit(
                Row::Field,
                Some(a),
                &sample,
                &field.name,
                pair.map(|f| f.value.as_str()),
            )?;
        }
        Ok(())
    }
}

fn run(cli: &Cli) -> io::Result<bool> {
    let Inputs {
        mut a,
        mut b,
        mut order,
        names_a,
        names_b,
        shared_samples,
        declared_types,
        header_differences,
    } = open_pair(&cli.a, &cli.b)?;
    let tolerances = Tolerances::new(&cli.tolerances, &declared_types)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;

    let mut report = Report {
        out: BufWriter::new(io::stdout().lock()),
        shared_samples,
        tolerances,
        ignore: Patterns::new(&cli.ignore),
        counts: Counts::default(),
    };
    writeln!(report.out, "{COLUMNS}")?;
    report.header(&header_differences)?;
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
        "vcf-parity: {} records compared, {} only in a, {} only in b, {} field differences, {} field differences within tolerance, {} samples in one file only, {} header differences",
        c.compared, c.only_a, c.only_b, c.fields, c.tolerated, c.samples, c.header
    );
    Ok(c.any())
}

/// Replaces each `@FILE` argument with the lines of FILE, skipping blank lines and `#` comments.
fn expand_argument_files() -> io::Result<Vec<OsString>> {
    let mut args = Vec::new();
    for arg in std::env::args_os() {
        let Some(path) = arg.to_str().and_then(|a| a.strip_prefix('@')) else {
            args.push(arg);
            continue;
        };
        let text = std::fs::read_to_string(path)
            .map_err(|e| io::Error::new(e.kind(), format!("{path}: {e}")))?;
        args.extend(
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(OsString::from),
        );
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match expand_argument_files() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("vcf-parity: {e}");
            return ExitCode::from(2);
        }
    };
    let cli = <Cli as clap::Parser>::parse_from(args);
    match run(&cli) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(1),
        Err(e) => {
            eprintln!("vcf-parity: {e}");
            ExitCode::from(2)
        }
    }
}
