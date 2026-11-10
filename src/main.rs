use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufWriter, Write};
use std::process::ExitCode;

use noodles::vcf;

const USAGE: &str = "usage: vcf-parity <a.vcf[.gz]> <b.vcf[.gz]>";
const COLUMNS: &str = "chrom\tpos\tref\talt\tsample\tfield\tkind\tvalue_a\tvalue_b";
const MISSING: &str = ".";

/// One VCF record, flattened to (sample, field, value) triples; sample is "." for site fields.
struct Record {
    key: (usize, usize),
    chrom: String,
    pos: usize,
    reference: String,
    alternate: String,
    fields: Vec<(String, String, String)>,
}

struct Input {
    path: String,
    reader: vcf::io::Reader<Box<dyn BufRead>>,
    header: vcf::Header,
    buf: vcf::Record,
    samples: Vec<(usize, String)>,
    next: Option<Record>,
    last_key: (usize, usize),
}

impl Input {
    fn open(path: &str) -> io::Result<Self> {
        let mut reader = vcf::io::reader::Builder::default()
            .build_from_path(path)
            .map_err(|e| with_path(path, e))?;
        let header = reader.read_header().map_err(|e| with_path(path, e))?;
        Ok(Self {
            path: path.to_string(),
            reader,
            header,
            buf: vcf::Record::default(),
            samples: Vec::new(),
            next: None,
            last_key: (0, 0),
        })
    }

    fn sample_names(&self) -> Vec<String> {
        self.header.sample_names().iter().cloned().collect()
    }

    fn read(&mut self, contigs: &mut HashMap<String, usize>) -> io::Result<Option<Record>> {
        if self
            .reader
            .read_record(&mut self.buf)
            .map_err(|e| with_path(&self.path, e))?
            == 0
        {
            return Ok(None);
        }
        let record = self
            .flatten(contigs)
            .map_err(|e| with_path(&self.path, e))?;
        if record.key < self.last_key {
            return Err(with_path(
                &self.path,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "records are not sorted at {}:{} (contig order follows the ##contig lines of the first file)",
                        record.chrom, record.pos
                    ),
                ),
            ));
        }
        self.last_key = record.key;
        Ok(Some(record))
    }

    /// Returns all records at the next position, or an empty block at end of file.
    fn block(&mut self, contigs: &mut HashMap<String, usize>) -> io::Result<Vec<Record>> {
        if self.next.is_none() {
            self.next = self.read(contigs)?;
        }
        let Some(first) = self.next.take() else {
            return Ok(Vec::new());
        };
        let key = first.key;
        let mut block = vec![first];
        loop {
            match self.read(contigs)? {
                Some(record) if record.key == key => block.push(record),
                other => {
                    self.next = other;
                    return Ok(block);
                }
            }
        }
    }

    fn flatten(&self, contigs: &mut HashMap<String, usize>) -> io::Result<Record> {
        let r = &self.buf;
        let chrom = r.reference_sequence_name().to_string();
        let pos = r.variant_start().transpose()?.map_or(0, usize::from);
        let next_index = contigs.len();
        let contig = *contigs.entry(chrom.clone()).or_insert(next_index);

        let site = |field: &str, value: &str| {
            let value = if value.is_empty() { MISSING } else { value };
            (MISSING.to_string(), field.to_string(), value.to_string())
        };
        let qual = r.quality_score().transpose()?.map(|q| q.to_string());
        let mut fields = vec![
            site("ID", r.ids().as_ref()),
            site("QUAL", qual.as_deref().unwrap_or(MISSING)),
            site("FILTER", r.filters().as_ref()),
        ];

        let info = r.info();
        for entry in info
            .as_ref()
            .split(';')
            .filter(|e| !e.is_empty() && *e != MISSING)
        {
            let (key, value) = entry.split_once('=').unwrap_or((entry, "true"));
            fields.push(site(&format!("INFO/{key}"), value));
        }

        let samples = r.samples();
        let keys: Vec<&str> = samples.keys().iter().collect();
        let values: Vec<_> = samples.iter().collect();
        for (index, name) in &self.samples {
            let Some(sample) = values.get(*index) else {
                continue;
            };
            let sample_values: Vec<&str> = sample.as_ref().split(':').collect();
            for (i, key) in keys.iter().enumerate() {
                let value = sample_values.get(i).copied().unwrap_or(MISSING);
                fields.push((name.clone(), format!("FORMAT/{key}"), value.to_string()));
            }
        }

        Ok(Record {
            key: (contig, pos),
            chrom,
            pos,
            reference: r.reference_bases().to_string(),
            alternate: r.alternate_bases().as_ref().to_string(),
            fields,
        })
    }
}

fn with_path(path: &str, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{path}: {e}"))
}

#[derive(Default)]
struct Counts {
    compared: usize,
    only_a: usize,
    only_b: usize,
    fields: usize,
}

struct Report<W: Write> {
    out: W,
    counts: Counts,
}

impl<W: Write> Report<W> {
    fn row(
        &mut self,
        at: &Record,
        sample: &str,
        field: &str,
        kind: &str,
        a: &str,
        b: &str,
    ) -> io::Result<()> {
        writeln!(
            self.out,
            "{}\t{}\t{}\t{}\t{sample}\t{field}\t{kind}\t{a}\t{b}",
            at.chrom, at.pos, at.reference, at.alternate
        )
    }

    fn only_in_a(&mut self, record: &Record) -> io::Result<()> {
        self.counts.only_a += 1;
        self.row(record, MISSING, "record", "missing_in_b", MISSING, MISSING)
    }

    fn only_in_b(&mut self, record: &Record) -> io::Result<()> {
        self.counts.only_b += 1;
        self.row(record, MISSING, "record", "missing_in_a", MISSING, MISSING)
    }

    fn compare(&mut self, a: &Record, b: &Record) -> io::Result<()> {
        self.counts.compared += 1;
        let b_values: HashMap<(&str, &str), &str> = b
            .fields
            .iter()
            .map(|(s, f, v)| ((s.as_str(), f.as_str()), v.as_str()))
            .collect();
        let mut seen = HashSet::new();
        for (sample, field, value) in &a.fields {
            seen.insert((sample.as_str(), field.as_str()));
            match b_values.get(&(sample.as_str(), field.as_str())) {
                Some(other) if other == value => {}
                Some(other) => {
                    self.counts.fields += 1;
                    self.row(a, sample, field, "differs", value, other)?;
                }
                None => {
                    self.counts.fields += 1;
                    self.row(a, sample, field, "missing_in_b", value, MISSING)?;
                }
            }
        }
        for (sample, field, value) in &b.fields {
            if !seen.contains(&(sample.as_str(), field.as_str())) {
                self.counts.fields += 1;
                self.row(a, sample, field, "missing_in_a", MISSING, value)?;
            }
        }
        Ok(())
    }

    /// Pairs records at the same position by REF and ALT, in file order.
    fn compare_blocks(&mut self, a: Vec<Record>, mut b: Vec<Record>) -> io::Result<()> {
        for record in a {
            match b
                .iter()
                .position(|o| o.reference == record.reference && o.alternate == record.alternate)
            {
                Some(i) => {
                    let other = b.remove(i);
                    self.compare(&record, &other)?;
                }
                None => self.only_in_a(&record)?,
            }
        }
        b.iter().try_for_each(|record| self.only_in_b(record))
    }
}

fn run(path_a: &str, path_b: &str) -> io::Result<bool> {
    let mut a = Input::open(path_a)?;
    let mut b = Input::open(path_b)?;

    let mut contigs: HashMap<String, usize> = HashMap::new();
    for name in a.header.contigs().keys().chain(b.header.contigs().keys()) {
        let next_index = contigs.len();
        contigs.entry(name.clone()).or_insert(next_index);
    }

    let names_a = a.sample_names();
    let names_b = b.sample_names();
    a.samples = names_a
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, n)| names_b.contains(n))
        .collect();
    b.samples = names_b
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, n)| names_a.contains(n))
        .collect();

    let mut report = Report {
        out: BufWriter::new(io::stdout().lock()),
        counts: Counts::default(),
    };
    writeln!(report.out, "{COLUMNS}")?;

    let mut sample_rows = 0;
    for (names, other, kind) in [
        (&names_a, &names_b, "missing_in_b"),
        (&names_b, &names_a, "missing_in_a"),
    ] {
        for name in names.iter().filter(|n| !other.contains(n)) {
            sample_rows += 1;
            let (va, vb) = if kind == "missing_in_b" {
                (name.as_str(), MISSING)
            } else {
                (MISSING, name.as_str())
            };
            writeln!(report.out, ".\t.\t.\t.\t{name}\tsample\t{kind}\t{va}\t{vb}")?;
        }
    }

    let mut block_a = a.block(&mut contigs)?;
    let mut block_b = b.block(&mut contigs)?;
    loop {
        match (
            block_a.first().map(|r| r.key),
            block_b.first().map(|r| r.key),
        ) {
            (None, None) => break,
            (Some(ka), Some(kb)) if ka == kb => {
                report.compare_blocks(block_a, block_b)?;
                block_a = a.block(&mut contigs)?;
                block_b = b.block(&mut contigs)?;
            }
            (Some(ka), kb) if kb.is_none_or(|kb| ka < kb) => {
                block_a.iter().try_for_each(|r| report.only_in_a(r))?;
                block_a = a.block(&mut contigs)?;
            }
            _ => {
                block_b.iter().try_for_each(|r| report.only_in_b(r))?;
                block_b = b.block(&mut contigs)?;
            }
        }
    }
    report.out.flush()?;

    let c = &report.counts;
    eprintln!(
        "vcf-parity: {} records compared, {} only in a, {} only in b, {} field differences, {} samples in one file only",
        c.compared, c.only_a, c.only_b, c.fields, sample_rows
    );
    Ok(c.only_a + c.only_b + c.fields + sample_rows > 0)
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
