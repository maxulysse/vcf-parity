# Overview

Accuracy tools ([hap.py](https://github.com/Illumina/hap.py), [RTG vcfeval](https://github.com/RealTimeGenomics/rtg-tools), [Truvari](https://github.com/ACEnglish/truvari), [Aardvark](https://github.com/PacificBiosciences/aardvark)) score calls against a truth set.
vcf-parity answers a different question: do two VCFs carry the same records with the same values?

vcf-parity reads only VCF files. It does not run variant callers or read alignments.
Typical pairs to compare:

- calls from a new caller and from the reference implementation it reproduces;
- calls made from BAM input and from CRAM input of the same reads;
- calls made on one platform and on another (for example x86_64 and ARM);
- calls from one release of a caller and from the previous release.

## Planned features

- Per-field tolerances: exact, absolute or relative.
