# Usage

```console
vcf-parity a.vcf.gz b.vcf.gz > discrepancies.tsv
vcf-parity --tolerance QUAL=abs:0.01 --tolerance INFO/AF=rel:1e-6 a.vcf.gz b.vcf.gz
vcf-parity --ignore header/fileDate --ignore ID a.vcf.gz b.vcf.gz
vcf-parity @reruns.args a.vcf.gz b.vcf.gz
vcf-parity --help
vcf-parity --version
```

## Inputs

VCF or bgzipped VCF, sorted by position.
Contig order comes from the `##contig` lines of the first file, then the second; contigs without a `##contig` line follow in order of first appearance.
Unsorted input is an error, including a second file sorted in another contig order.

## Matching and comparison

Records are matched on CHROM, POS, REF and ALT.
Matched records are compared on ID, QUAL, FILTER, every INFO key, and every FORMAT key of the samples present in both files.
ID and FILTER are compared as sets: order and repeats do not count, and their items are written in sorted order.
Other values are compared as text unless a [tolerance](#tolerances) applies.
Samples present in one file only are reported, not compared.

## Headers

Every `##` line is compared.
A line of the form `##KEY=<ID=X,...>` is matched on `KEY` and `ID`, then compared attribute by attribute, for example `Number`, `Type` or `Description`.
Other lines, such as `##fileformat` or `##fileDate`, are matched on `KEY` and compared as text.
The order of the contigs declared in both files is compared as `header/contig`.

## Ignored fields

`--ignore PATTERN` skips every field whose name in the [output](output.md) matches `PATTERN`, and can be repeated.
A pattern is a list of `/`-separated segments: `*` matches any one segment, and a pattern also matches every field below it.

| `PATTERN`                | Skips                                            |
| ------------------------ | ------------------------------------------------ |
| `ID`                     | record IDs                                       |
| `FORMAT`                 | every `FORMAT` key                               |
| `header/fileDate`        | the `##fileDate` line                            |
| `header/*/*/Description` | the `Description` of every `##KEY=<ID=...>` line |
| `header`                 | the whole header                                 |
| `record`, `sample`       | records or samples present in one file only      |

Ignored fields are not written to the TSV, do not change the exit code and are not counted in the summary.

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

## Argument files

`@FILE` is replaced by the lines of `FILE`, one argument per line; blank lines and lines starting with `#` are skipped.
An argument file keeps the `--ignore` and `--tolerance` options of one kind of comparison together:

```text
# two runs of the same caller
--ignore=header/fileDate
--ignore=header/source
--tolerance=QUAL=abs:0.01
```

## Exit codes

| Code | Meaning      |
| ---- | ------------ |
| `0`  | files match  |
| `1`  | files differ |
| `2`  | error        |

A summary goes to stderr: records compared, records and samples in one file only, field differences, differences within tolerance, and header differences.

If a reader such as `head` closes stdout early, vcf-parity stops without a message or summary and exits with `1` if it found a difference, `0` otherwise.

## Performance

Comparing the GIAB HG002 v4.2.1 benchmark (GRCh38, chromosomes 1 to 22, 4,048,342 records, one sample) with itself takes about 20 s and 3.4 MB of memory with vcf-parity 0.2.1 on a 16-core Linux workstation.
Input is streamed on one thread, so memory does not grow with file size.

```console
curl -O https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/release/AshkenazimTrio/HG002_NA24385_son/NISTv4.2.1/GRCh38/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz
vcf-parity HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz > /dev/null
```
