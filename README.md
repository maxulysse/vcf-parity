# vcf-parity

Field-level comparison of two VCF files.

Accuracy tools ([hap.py](https://github.com/Illumina/hap.py), [RTG vcfeval](https://github.com/RealTimeGenomics/rtg-tools), [Truvari](https://github.com/ACEnglish/truvari), [Aardvark](https://github.com/PacificBiosciences/aardvark)) score calls against a truth set.
vcf-parity answers a different question: do two VCFs carry the same records with the same values?

vcf-parity reads only VCF files. It does not run variant callers or read alignments.
Typical pairs to compare:

- calls from a new caller and from the reference implementation it reproduces;
- calls made from BAM input and from CRAM input of the same reads;
- calls made on one platform and on another (for example x86_64 and ARM);
- calls from one release of a caller and from the previous release.

## Usage

```console
vcf-parity a.vcf.gz b.vcf.gz > discrepancies.tsv
```

Inputs are VCF or bgzipped VCF, sorted by position in the contig order of the `##contig` header lines.
Records are matched on CHROM, POS, REF and ALT.
For each matched pair, ID, QUAL, FILTER, every INFO key and every FORMAT key of each sample present in both files are compared as text.

The output is one TSV row per discrepancy:

```tsv
chrom  pos   ref  alt    sample  field           kind          value_a  value_b
.      .     .    .      S2      sample          missing_in_a  .        S2
chr20  2000  G    <INS>  .       QUAL            differs       30       31
chr20  2000  G    <INS>  .       INFO/IMPRECISE  missing_in_b  true     .
chr20  2000  G    <INS>  S1      FORMAT/GT       differs       1/1      0/1
chr20  3000  C    <DEL>  .       record          missing_in_b  .        .
```

`sample` is `.` for site fields. INFO flags have the value `true`.
A summary goes to stderr.
The exit code is `0` when the files match, `1` when they differ and `2` on error.

## Planned features

- Per-field tolerances: exact, absolute or relative.

Status: early development.

## License

[MIT](LICENSE)
