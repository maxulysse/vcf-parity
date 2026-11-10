# vcf-parity: Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0](https://github.com/maxulysse/vcf-parity/releases/tag/0.1.0) - Dohko

Dohko is the original Libra Gold Saint in Saint Seiya, keeper of the Libra Cloth.

### Added

- [#1](https://github.com/maxulysse/vcf-parity/pull/1) - Field-level comparison of two sorted VCF or bgzipped VCF files: records matched on CHROM, POS, REF and ALT; ID, QUAL, FILTER, INFO and FORMAT of shared samples compared as text
- [#1](https://github.com/maxulysse/vcf-parity/pull/1) - TSV discrepancy log on stdout, summary on stderr, exit codes `0` match, `1` differ, `2` error
- [#2](https://github.com/maxulysse/vcf-parity/pull/2) - `--help` and `--version`
