# Usage

```console
vcf-parity a.vcf.gz b.vcf.gz > discrepancies.tsv
vcf-parity --tolerance QUAL=abs:0.01 --tolerance INFO/AF=rel:1e-6 a.vcf.gz b.vcf.gz
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

## Tolerances

By default values must have identical text.
`--tolerance FIELD=SPEC` relaxes this for one numeric field and can be repeated.
`FIELD` is named as in the [output](output.md): `QUAL`, `INFO/<key>` or `FORMAT/<key>`; a `FORMAT` tolerance applies to every shared sample.

| `SPEC`        | Values match when                                                  |
| ------------- | ------------------------------------------------------------------ |
| `exact`       | they are the same number, for example `0.5` and `0.50`             |
| `abs:X`       | \|a − b\| ≤ X                                                      |
| `rel:X`       | \|a − b\| ≤ X × max(\|a\|, \|b\|)                                  |
| `abs:X,rel:Y` | either bound holds; the absolute bound covers values close to zero |

Lists such as `AD` or `PL` match element by element and must have the same length.
A missing value (`.`) or a value that is not a number matches only identical text.

`INFO` and `FORMAT` fields must be declared `Integer` or `Float` in the headers; a tolerance on `ID`, `FILTER` or a field of another type is an error.
A field declared in neither header gives a warning and the tolerance still applies.
Differences within tolerance are not written to the TSV and do not change the exit code; the summary on stderr counts them.

## Exit codes

| Code | Meaning      |
| ---- | ------------ |
| `0`  | files match  |
| `1`  | files differ |
| `2`  | error        |

A summary goes to stderr: records compared, records and samples in one file only, field differences, and differences within tolerance.
