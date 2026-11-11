<img src="docs/images/logo.svg" alt="vcf-parity logo" width="128">

# vcf-parity

Compares two VCF files record by record and field by field.

Accuracy tools ([hap.py](https://github.com/Illumina/hap.py), [RTG vcfeval](https://github.com/RealTimeGenomics/rtg-tools), [Truvari](https://github.com/ACEnglish/truvari), [Aardvark](https://github.com/PacificBiosciences/aardvark)) score calls against a truth set.
vcf-parity checks whether two VCFs carry the same records with the same values.
It reads VCF files only: it does not run callers or read alignments.

Typical pairs of VCFs, from:

- a caller and the reference implementation it reproduces;
- BAM and CRAM input of the same reads;
- two platforms, for example x86_64 and ARM;
- two releases of a caller.

## Install

```console
cargo install vcf-parity
```

To install the development version from `main`:

```console
cargo install --git https://github.com/maxulysse/vcf-parity
```

## Usage

```console
vcf-parity a.vcf.gz b.vcf.gz > discrepancies.tsv
```

Headers are compared line by line, records are matched on CHROM, POS, REF and ALT, numeric fields accept tolerances, and any field can be ignored.
Each discrepancy is one TSV row, and the exit code is `1` when the files differ.
See [usage](docs/usage.md) and [output](docs/output.md).

Early development. [MIT](LICENSE) license.
