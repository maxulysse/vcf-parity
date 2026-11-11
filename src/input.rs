use std::collections::HashMap;
use std::io::{self, BufRead};

use noodles::vcf;

use crate::MISSING;
use crate::header;

/// Whether a field belongs to the site or to a shared sample (index into the shared sample names).
#[derive(Clone, Copy, PartialEq)]
pub enum Scope {
    Site,
    Sample(usize),
}

pub struct Field {
    pub scope: Scope,
    pub name: String,
    pub value: String,
}

pub struct Record {
    pub contig: usize,
    pub chrom: String,
    pub pos: usize,
    pub reference: String,
    pub alternate: String,
    pub fields: Vec<Field>,
}

impl Record {
    pub fn key(&self) -> (usize, usize) {
        (self.contig, self.pos)
    }

    pub fn same_alleles(&self, other: &Record) -> bool {
        self.reference == other.reference && self.alternate == other.alternate
    }
}

/// Contig order for merging and sort checks: `##contig` lines of a, then of b, then first appearance in the records.
pub struct ContigOrder {
    indices: HashMap<String, usize>,
    /// Contigs below this index come from `##contig` lines; the others are numbered as first read.
    declared: usize,
    /// The `##contig` lines of b are out of this order, so b's records can look unsorted.
    headers_disagree: bool,
}

impl ContigOrder {
    fn new(a: &vcf::Header, b: &vcf::Header) -> Self {
        let mut indices = HashMap::new();
        for name in a.contigs().keys().chain(b.contigs().keys()) {
            let i = indices.len();
            indices.entry(name.clone()).or_insert(i);
        }
        let headers_disagree = !b.contigs().keys().map(|name| indices[name]).is_sorted();
        ContigOrder {
            declared: indices.len(),
            indices,
            headers_disagree,
        }
    }

    /// Extra text for an unsorted-records error between contigs `previous` and `current`.
    fn explain(&self, previous: usize, current: usize) -> String {
        let mut text = String::new();
        if self.headers_disagree {
            text.push_str("; the ##contig lines of the two files list contigs in different orders");
        }
        if previous.max(current) >= self.declared {
            text.push_str(
                "; contigs without a ##contig line are ordered as first read, add ##contig lines",
            );
        }
        text
    }

    fn index(&mut self, name: &str) -> usize {
        if let Some(&i) = self.indices.get(name) {
            return i;
        }
        let i = self.indices.len();
        self.indices.insert(name.to_string(), i);
        i
    }
}

pub struct Input {
    path: String,
    reader: Box<dyn BufRead>,
    /// The current record line.
    line: String,
    /// Column of each shared sample in this file.
    sample_columns: Vec<usize>,
    next: Option<Record>,
    last_key: (usize, usize),
}

/// Both inputs, opened together: contig order and shared samples depend on both headers.
pub struct Inputs {
    pub a: Input,
    pub b: Input,
    pub order: ContigOrder,
    pub names_a: Vec<String>,
    pub names_b: Vec<String>,
    pub shared_samples: Vec<String>,
    /// Header types of each INFO and FORMAT key, from both files, keyed as `INFO/<key>` or `FORMAT/<key>`.
    pub declared_types: HashMap<String, Vec<String>>,
    pub header_differences: Vec<header::Difference>,
}

pub fn open_pair(path_a: &str, path_b: &str) -> io::Result<Inputs> {
    let (reader_a, header_a) = open(path_a)?;
    let (reader_b, header_b) = open(path_b)?;

    let header_differences = header::differences(&header_a, &header_b)?;
    let order = ContigOrder::new(&header_a, &header_b);

    let mut declared_types = HashMap::new();
    for header in [&header_a, &header_b] {
        declare_types(header, &mut declared_types);
    }

    let names = |h: &vcf::Header| -> Vec<String> { h.sample_names().iter().cloned().collect() };
    let (names_a, names_b) = (names(&header_a), names(&header_b));
    let shared_samples: Vec<String> = names_a
        .iter()
        .filter(|n| names_b.contains(n))
        .cloned()
        .collect();
    let input = |path: &str, reader, names: &[String]| Input {
        path: path.to_string(),
        reader,
        line: String::new(),
        sample_columns: shared_samples
            .iter()
            .map(|s| names.iter().position(|n| n == s).unwrap())
            .collect(),
        next: None,
        last_key: (0, 0),
    };
    Ok(Inputs {
        a: input(path_a, reader_a, &names_a),
        b: input(path_b, reader_b, &names_b),
        order,
        names_a,
        names_b,
        shared_samples,
        declared_types,
        header_differences,
    })
}

fn declare_types(header: &vcf::Header, types: &mut HashMap<String, Vec<String>>) {
    for (key, info) in header.infos() {
        types
            .entry(format!("INFO/{key}"))
            .or_default()
            .push(info.ty().as_ref().to_string());
    }
    for (key, format) in header.formats() {
        types
            .entry(format!("FORMAT/{key}"))
            .or_default()
            .push(format.ty().as_ref().to_string());
    }
}

/// Reads the header with noodles; records are split by `Input::read`, so values keep their exact text.
fn open(path: &str) -> io::Result<(Box<dyn BufRead>, vcf::Header)> {
    let mut reader = vcf::io::reader::Builder::default()
        .build_from_path(path)
        .map_err(|e| with_path(path, e))?;
    let header = reader.read_header().map_err(|e| with_path(path, e))?;
    Ok((reader.into_inner(), header))
}

fn with_path(path: &str, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{path}: {e}"))
}

impl Input {
    fn read(&mut self, order: &mut ContigOrder) -> io::Result<Option<Record>> {
        let path = &self.path;
        self.line.clear();
        if self
            .reader
            .read_line(&mut self.line)
            .map_err(|e| with_path(path, e))?
            == 0
        {
            return Ok(None);
        }
        let columns: Vec<&str> = self
            .line
            .trim_end_matches(['\n', '\r'])
            .split('\t')
            .collect();
        let record =
            flatten(&columns, order, &self.sample_columns).map_err(|e| with_path(path, e))?;
        if record.key() < self.last_key {
            let message = format!(
                "records are not sorted at {}:{} (contig order follows the ##contig lines of the first file){}",
                record.chrom,
                record.pos,
                order.explain(self.last_key.0, record.contig)
            );
            return Err(with_path(
                path,
                io::Error::new(io::ErrorKind::InvalidData, message),
            ));
        }
        self.last_key = record.key();
        Ok(Some(record))
    }

    /// Returns all records at the next position, or an empty block at end of file.
    pub fn block(&mut self, order: &mut ContigOrder) -> io::Result<Vec<Record>> {
        if self.next.is_none() {
            self.next = self.read(order)?;
        }
        let Some(first) = self.next.take() else {
            return Ok(Vec::new());
        };
        let key = first.key();
        let mut block = vec![first];
        loop {
            match self.read(order)? {
                Some(record) if record.key() == key => block.push(record),
                other => {
                    self.next = other;
                    return Ok(block);
                }
            }
        }
    }
}

/// Sorts and deduplicates a `;`-separated list, so order and repeats are not differences.
fn as_set(list: &str) -> String {
    let mut items: Vec<&str> = list.split(';').collect();
    items.sort_unstable();
    items.dedup();
    items.join(";")
}

/// Splits one record line, already cut into its tab-separated columns, into comparable fields.
fn flatten(
    columns: &[&str],
    order: &mut ContigOrder,
    sample_columns: &[usize],
) -> io::Result<Record> {
    let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
    let &[
        chrom,
        pos,
        ids,
        reference,
        alternate,
        qual,
        filters,
        info,
        ref samples @ ..,
    ] = columns
    else {
        return Err(invalid(format!(
            "expected at least 8 columns, got {}",
            columns.len()
        )));
    };
    let pos: usize = pos
        .parse()
        .map_err(|_| invalid(format!("{chrom}: invalid POS '{pos}'")))?;
    let site = |name: &str, value: &str| Field {
        scope: Scope::Site,
        name: name.to_string(),
        value: if value.is_empty() { MISSING } else { value }.to_string(),
    };
    let mut fields = vec![
        site("ID", &as_set(ids)),
        site("QUAL", qual),
        site("FILTER", &as_set(filters)),
    ];

    for entry in info.split(';').filter(|e| !e.is_empty() && *e != MISSING) {
        let (key, value) = entry.split_once('=').unwrap_or((entry, "true"));
        fields.push(site(&format!("INFO/{key}"), value));
    }

    let keys: Vec<&str> = samples
        .first()
        .map_or(Vec::new(), |k| k.split(':').collect());
    for (shared, &column) in sample_columns.iter().enumerate() {
        let sample = samples.get(1 + column).ok_or_else(|| {
            invalid(format!(
                "{chrom}:{pos}: fewer sample columns than in the header"
            ))
        })?;
        let sample_values: Vec<&str> = sample.split(':').collect();
        for (i, key) in keys.iter().enumerate() {
            fields.push(Field {
                scope: Scope::Sample(shared),
                name: format!("FORMAT/{key}"),
                value: sample_values.get(i).copied().unwrap_or(MISSING).to_string(),
            });
        }
    }

    Ok(Record {
        contig: order.index(chrom),
        chrom: chrom.to_string(),
        pos,
        reference: reference.to_string(),
        alternate: alternate.to_string(),
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(lines: &str) -> vcf::Header {
        format!("##fileformat=VCFv4.3\n{lines}#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n")
            .parse()
            .unwrap()
    }

    #[test]
    fn rejects_lines_with_fewer_than_8_columns() {
        let mut order = ContigOrder::new(&header(""), &header(""));
        let error = flatten(&["chr1", "1", ".", "A", "C", "5", "PASS"], &mut order, &[])
            .err()
            .unwrap();
        assert!(
            error
                .to_string()
                .contains("expected at least 8 columns, got 7")
        );
    }

    #[test]
    fn keeps_header_types_from_both_files() {
        let a = header(
            "##INFO=<ID=AF,Number=A,Type=Float,Description=\"x\">\n\
             ##FORMAT=<ID=XD,Number=1,Type=Integer,Description=\"x\">\n",
        );
        let b = header("##FORMAT=<ID=XD,Number=1,Type=String,Description=\"x\">\n");
        let mut types = HashMap::new();
        for h in [&a, &b] {
            declare_types(h, &mut types);
        }
        assert_eq!(types["INFO/AF"], ["Float"]);
        assert_eq!(types["FORMAT/XD"], ["Integer", "String"]);
    }
}
