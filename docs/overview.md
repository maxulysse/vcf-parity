# Overview

Accuracy tools ([hap.py](https://github.com/Illumina/hap.py), [RTG vcfeval](https://github.com/RealTimeGenomics/rtg-tools), [Truvari](https://github.com/ACEnglish/truvari), [Aardvark](https://github.com/PacificBiosciences/aardvark)) score calls against a truth set.
vcf-parity checks whether two VCFs carry the same records with the same values.
It reads VCF files only: it does not run callers or read alignments.

Typical pairs:

- a caller and the reference implementation it reproduces;
- calls from BAM and from CRAM input of the same reads;
- calls from two platforms, for example x86_64 and ARM;
- calls from two releases of a caller.

## Planned

- Per-field tolerances: exact, absolute or relative.
