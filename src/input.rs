use std::collections::HashMap;
use std::io::{self, BufRead};

use noodles::vcf;

use crate::MISSING;

/// Whether a field belongs to the site or to a shared sample (index into the shared sample names).
#[derive(Clone, Copy, PartialEq)]
pub enum Scope {
    Site,
    Sample(usize),
}

pub struct Record {
    pub contig: usize,
    pub chrom: String,
    pub pos: usize,
    pub reference: String,
    pub alternate: String,
    pub fields: Vec<((Scope, String), String)>,
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
pub struct ContigOrder(pub HashMap<String, usize>);

impl ContigOrder {
    pub fn index(&mut self, name: &str) -> usize {
        if let Some(&i) = self.0.get(name) {
            return i;
        }
        let i = self.0.len();
        self.0.insert(name.to_string(), i);
        i
    }
}

pub struct Input {
    pub path: String,
    pub reader: vcf::io::Reader<Box<dyn BufRead>>,
    pub buf: vcf::Record,
    /// Column of each shared sample in this file.
    pub sample_columns: Vec<usize>,
    pub next: Option<Record>,
    pub last_key: (usize, usize),
}

pub fn open(path: &str) -> io::Result<(vcf::io::Reader<Box<dyn BufRead>>, vcf::Header)> {
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
    pub fn read(&mut self, order: &mut ContigOrder) -> io::Result<Option<Record>> {
        let path = &self.path;
        if self
            .reader
            .read_record(&mut self.buf)
            .map_err(|e| with_path(path, e))?
            == 0
        {
            return Ok(None);
        }
        let record =
            flatten(&self.buf, &self.sample_columns, order).map_err(|e| with_path(path, e))?;
        if record.key() < self.last_key {
            let message = format!(
                "records are not sorted at {}:{} (contig order follows the ##contig lines of the first file)",
                record.chrom, record.pos
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

pub fn flatten(
    r: &vcf::Record,
    sample_columns: &[usize],
    order: &mut ContigOrder,
) -> io::Result<Record> {
    let site = |field: &str, value: &str| {
        let value = if value.is_empty() { MISSING } else { value };
        ((Scope::Site, field.to_string()), value.to_string())
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
        let Some(sample) = values.get(column) else {
            continue;
        };
        let sample_values: Vec<&str> = sample.as_ref().split(':').collect();
        for (i, key) in keys.iter().enumerate() {
            let value = sample_values.get(i).copied().unwrap_or(MISSING);
            fields.push((
                (Scope::Sample(shared), format!("FORMAT/{key}")),
                value.to_string(),
            ));
        }
    }

    let chrom = r.reference_sequence_name();
    Ok(Record {
        contig: order.index(chrom),
        chrom: chrom.to_string(),
        pos: r.variant_start().transpose()?.map_or(0, usize::from),
        reference: r.reference_bases().to_string(),
        alternate: r.alternate_bases().as_ref().to_string(),
        fields,
    })
}
