<img src="docs/images/logo.svg" alt="vcf-parity logo" width="96" align="right">

# vcf-parity

Compares two VCF files record by record and field by field.

```console
vcf-parity a.vcf.gz b.vcf.gz > discrepancies.tsv
```

Exit code `0` when the files match, `1` when they differ, `2` on error.
See the [documentation](docs/README.md).

Early development. [MIT](LICENSE) license.
