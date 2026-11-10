# Usage

```console
vcf-parity a.vcf.gz b.vcf.gz > discrepancies.tsv
vcf-parity --help
vcf-parity --version
```

## Inputs

VCF or bgzipped VCF, sorted by position.
Contig order comes from the `##contig` lines of the first file, then the second; contigs without a `##contig` line follow in order of first appearance.
Unsorted input is an error.

## Matching and comparison

Records are matched on CHROM, POS, REF and ALT.
Matched records are compared as text on ID, QUAL, FILTER, every INFO key, and every FORMAT key of the samples present in both files.
Samples present in one file only are reported, not compared.

## Exit codes

| Code | Meaning      |
| ---- | ------------ |
| `0`  | files match  |
| `1`  | files differ |
| `2`  | error        |

A summary goes to stderr.
