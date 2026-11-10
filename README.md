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

## Planned features

- Match records between the two files.
- Compare QUAL, FILTER and every INFO and FORMAT field.
- Per-field tolerances: exact, absolute or relative.
- Machine-readable discrepancy log and a non-zero exit code when files differ beyond tolerance.

Status: early development, nothing usable yet.

## License

[MIT](LICENSE)
