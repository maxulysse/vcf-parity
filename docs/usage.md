# Usage

```console
vcf-parity a.vcf.gz b.vcf.gz > discrepancies.tsv
```

## Inputs

VCF or bgzipped VCF, sorted by position in the contig order of the `##contig` header lines.
Unsorted input is an error.

## Matching and comparison

Records are matched on CHROM, POS, REF and ALT.
For each matched pair, ID, QUAL, FILTER, every INFO key and every FORMAT key of each sample present in both files are compared as text.
Samples present in only one file are reported, not compared.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | files match |
| `1` | files differ |
| `2` | error |

A summary goes to stderr.
