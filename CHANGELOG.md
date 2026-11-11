# vcf-parity: Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- [#6](https://github.com/maxulysse/vcf-parity/pull/6) - Header comparison: `##KEY=<ID=...>` lines attribute by attribute, other `##` lines as text, order of shared contigs
- [#6](https://github.com/maxulysse/vcf-parity/pull/6) - `--ignore PATTERN` to skip record or header fields, with `*` matching one path segment
- [#6](https://github.com/maxulysse/vcf-parity/pull/6) - `@FILE` argument files, one argument per line
- [#4](https://github.com/maxulysse/vcf-parity/pull/4) - `--tolerance FIELD=SPEC` for numeric fields: `exact`, `abs:X`, `rel:X` or `abs:X,rel:Y`, element by element for lists; checked against the header types; matches within tolerance counted in the summary

### Changed

- ID compared as a set, like FILTER
- [#7](https://github.com/maxulysse/vcf-parity/pull/7) - FILTER compared as a set: order and repeats ignored, filters written in sorted order

## [0.1.0](https://github.com/maxulysse/vcf-parity/releases/tag/0.1.0) - Dohko

Dohko is the original Libra Gold Saint in Saint Seiya, keeper of the Libra Cloth.

### Added

- [#1](https://github.com/maxulysse/vcf-parity/pull/1) - Field-level comparison of two sorted VCF or bgzipped VCF files: records matched on CHROM, POS, REF and ALT; ID, QUAL, FILTER, INFO and FORMAT of shared samples compared as text
- [#1](https://github.com/maxulysse/vcf-parity/pull/1) - TSV discrepancy log on stdout, summary on stderr, exit codes `0` match, `1` differ, `2` error
- [#2](https://github.com/maxulysse/vcf-parity/pull/2) - `--help` and `--version`
