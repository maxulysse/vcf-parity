# vcf-parity

Field-level comparison of two VCF files.

Accuracy tools (hap.py, RTG vcfeval, Truvari, Aardvark) score calls against a truth set.
vcf-parity answers a different question: do two VCFs carry the same records with the same values?
Use it to compare a caller against a reference implementation, BAM against CRAM input, one platform against another, or a release against the previous one.

## Planned features

- Match records between the two files.
- Compare QUAL, FILTER and every INFO and FORMAT field.
- Per-field tolerances: exact, absolute or relative.
- Machine-readable discrepancy log and a non-zero exit code when files differ beyond tolerance.

Status: early development, nothing usable yet.

## License

[MIT](LICENSE)
