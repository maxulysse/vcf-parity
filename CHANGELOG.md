# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-10

### Added

- Field-level comparison of two sorted VCF or bgzipped VCF files: records matched on CHROM, POS, REF and ALT; ID, QUAL, FILTER, INFO and FORMAT of shared samples compared as text.
- TSV discrepancy log on stdout, summary on stderr, exit codes `0` match, `1` differ, `2` error.
- `--help` and `--version`.

[unreleased]: https://github.com/maxulysse/vcf-parity/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/maxulysse/vcf-parity/releases/tag/v0.1.0
