use std::io;

use noodles::vcf;

use crate::pair::{Pair, pair_by};

/// One header discrepancy: the output field and the values in each file.
pub struct Difference {
    pub field: String,
    pub values: Pair<String>,
}

/// Differences between two headers, including the order of the contigs declared in both.
pub fn differences(a: &vcf::Header, b: &vcf::Header) -> io::Result<Vec<Difference>> {
    let (lines_a, lines_b) = (lines(a)?, lines(b)?);
    let mut rows = Vec::new();
    let mut push = |field: String, values: Pair<String>| rows.push(Difference { field, values });
    for pair in pair_by(&lines_a, &lines_b, |x, y| x.path == y.path) {
        let Pair::Both(x, y) = pair else {
            push(pair.either().path.clone(), pair.map(|l| l.text.clone()));
            continue;
        };
        if x.attributes.is_empty() || y.attributes.is_empty() {
            if x.text != y.text {
                push(x.path.clone(), Pair::Both(x.text.clone(), y.text.clone()));
            }
            continue;
        }
        for attribute in pair_by(&x.attributes, &y.attributes, |m, n| m.0 == n.0) {
            if let Pair::Both(m, n) = attribute
                && m.1 == n.1
            {
                continue;
            }
            let field = format!("{}/{}", x.path, attribute.either().0);
            push(field, attribute.map(|(_, v)| v.clone()));
        }
    }
    let (contigs_a, contigs_b) = shared_contigs(a, b);
    if contigs_a != contigs_b {
        push(
            "header/contig".to_string(),
            Pair::Both(contigs_a.join(","), contigs_b.join(",")),
        );
    }
    Ok(rows)
}

/// One `##` header line: `##KEY=<ID=X,...>` has path `header/KEY/X` and its other attributes,
/// any other line has path `header/KEY` and no attributes.
struct Line {
    path: String,
    text: String,
    attributes: Vec<(String, String)>,
}

/// The `##` lines of a header, as noodles writes them back.
fn lines(header: &vcf::Header) -> io::Result<Vec<Line>> {
    let mut writer = vcf::io::Writer::new(Vec::new());
    writer.write_header(header)?;
    Ok(String::from_utf8_lossy(&writer.into_inner())
        .lines()
        .filter_map(|l| l.strip_prefix("##"))
        .map(line)
        .collect())
}

fn line(l: &str) -> Line {
    let (key, text) = l.split_once('=').unwrap_or((l, ""));
    let text = text.to_string();
    let structured = text
        .strip_prefix('<')
        .and_then(|t| t.strip_suffix('>'))
        .map(attributes);
    match structured {
        Some(mut attributes) if attributes.first().is_some_and(|(k, _)| k == "ID") => {
            let (_, id) = attributes.remove(0);
            Line {
                path: format!("header/{key}/{id}"),
                text,
                attributes,
            }
        }
        _ => Line {
            path: format!("header/{key}"),
            text,
            attributes: Vec::new(),
        },
    }
}

/// Splits `K=V,K="V, with \"quotes\"",...` into unquoted pairs.
fn attributes(s: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let (mut key, mut value) = (String::new(), String::new());
    let (mut in_value, mut quoted, mut escaped) = (false, false, false);
    for c in s.chars() {
        match c {
            _ if escaped => {
                value.push(c);
                escaped = false;
            }
            '\\' if quoted => escaped = true,
            '"' if in_value => quoted = !quoted,
            '=' if !in_value => in_value = true,
            ',' if !quoted => {
                pairs.push((std::mem::take(&mut key), std::mem::take(&mut value)));
                in_value = false;
            }
            _ if in_value => value.push(c),
            _ => key.push(c),
        }
    }
    pairs.push((key, value));
    pairs
}

/// Contigs declared in both headers, in the order of each.
fn shared_contigs<'h>(a: &'h vcf::Header, b: &'h vcf::Header) -> (Vec<&'h str>, Vec<&'h str>) {
    let in_other = |h: &'h vcf::Header, other: &vcf::Header| -> Vec<&'h str> {
        h.contigs()
            .keys()
            .filter(|k| other.contigs().contains_key(*k))
            .map(String::as_str)
            .collect()
    };
    (in_other(a, b), in_other(b, a))
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
    fn reports_attributes_lines_and_contig_order() {
        let a = header(
            "##fileDate=20260101\n\
             ##INFO=<ID=XD,Number=1,Type=Integer,Description=\"x\">\n\
             ##contig=<ID=chr1>\n##contig=<ID=chr2>\n",
        );
        let b = header(
            "##INFO=<ID=XD,Number=1,Type=Float,Description=\"x\">\n\
             ##FILTER=<ID=q10,Description=\"Low\">\n\
             ##contig=<ID=chr2>\n##contig=<ID=chr1>\n",
        );
        let rows: Vec<String> = differences(&a, &b)
            .unwrap()
            .into_iter()
            .map(|Difference { field, values }| match values {
                Pair::Both(x, y) => format!("{field} {x} {y}"),
                Pair::OnlyA(x) => format!("{field} {x} ."),
                Pair::OnlyB(y) => format!("{field} . {y}"),
            })
            .collect();
        assert_eq!(
            rows,
            [
                "header/INFO/XD/Type Integer Float",
                "header/fileDate 20260101 .",
                "header/FILTER/q10 . <ID=q10,Description=\"Low\">",
                "header/contig chr1,chr2 chr2,chr1",
            ]
        );
    }

    #[test]
    fn splits_structured_lines() {
        let l = line(r#"INFO=<ID=DP,Number=1,Type=Integer,Description="Depth, \"raw\"">"#);
        assert_eq!(l.path, "header/INFO/DP");
        assert_eq!(
            l.attributes,
            [
                ("Number".into(), "1".into()),
                ("Type".into(), "Integer".into()),
                ("Description".into(), r#"Depth, "raw""#.into()),
            ]
        );
    }

    #[test]
    fn keeps_other_lines_as_text() {
        for (l, path, text) in [
            ("fileDate=20160824", "header/fileDate", "20160824"),
            (
                "LibraryStats=<TIDDIT-3.6.0 Coverage=10>",
                "header/LibraryStats",
                "<TIDDIT-3.6.0 Coverage=10>",
            ),
        ] {
            let l = line(l);
            assert_eq!((l.path.as_str(), l.text.as_str()), (path, text));
            assert!(l.attributes.is_empty());
        }
    }
}
