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
            indices,
            headers_disagree,
        }
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
    reader: vcf::io::Reader<Box<dyn BufRead>>,
    buf: vcf::Record,
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
        buf: vcf::Record::default(),
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

fn open(path: &str) -> io::Result<(vcf::io::Reader<Box<dyn BufRead>>, vcf::Header)> {
    let mut reader = vcf::io::reader::Builder::default()
        .build_from_path(path)
        .map_err(|e| with_path(path, e))?;
    let header = reader.read_header().map_err(|e| with_path(path, e))?;
    Ok((reader, header))
}

fn with_path(path: &str, e: io::Error) -> io::Error {
    io::Error::new(e.kind(), format!("{path}: {e}"))
}

impl Input {
    fn read(&mut self, order: &mut ContigOrder) -> io::Result<Option<Record>> {
        let path = &self.path;
        if self
            .reader
            .read_record(&mut self.buf)
            .map_err(|e| with_path(path, e))?
            == 0
        {
            return Ok(None);
        }
        let contig = order.index(self.buf.reference_sequence_name());
        let record =
            flatten(&self.buf, contig, &self.sample_columns).map_err(|e| with_path(path, e))?;
        if record.key() < self.last_key {
            let mut message = format!(
                "records are not sorted at {}:{} (contig order follows the ##contig lines of the first file)",
                record.chrom, record.pos
            );
            if order.headers_disagree {
                message.push_str(
                    "; the ##contig lines of the two files list contigs in different orders",
                );
            }
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

fn flatten(r: &vcf::Record, contig: usize, sample_columns: &[usize]) -> io::Result<Record> {
    let chrom = r.reference_sequence_name();
    let pos = r.variant_start().transpose()?.map_or(0, usize::from);
    let site = |name: &str, value: &str| Field {
        scope: Scope::Site,
        name: name.to_string(),
        value: if value.is_empty() { MISSING } else { value }.to_string(),
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
    for (shared, &column) in sample_columns.iter().enumerate() {
        let sample = values.get(column).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{chrom}:{pos}: fewer sample columns than in the header"),
            )
        })?;
        let sample_values: Vec<&str> = sample.as_ref().split(':').collect();
        for (i, key) in keys.iter().enumerate() {
            fields.push(Field {
                scope: Scope::Sample(shared),
                name: format!("FORMAT/{key}"),
                value: sample_values.get(i).copied().unwrap_or(MISSING).to_string(),
            });
        }
    }

    Ok(Record {
        contig,
        chrom: chrom.to_string(),
        pos,
        reference: r.reference_bases().to_string(),
        alternate: r.alternate_bases().as_ref().to_string(),
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
