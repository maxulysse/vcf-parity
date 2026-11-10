# Output

vcf-parity writes one TSV row per discrepancy to stdout, after a header row:

```tsv
chrom  pos   ref  alt    sample  field           kind          value_a  value_b
.      .     .    .      S2      sample          missing_in_a  .        S2
chr20  2000  G    <INS>  .       QUAL            differs       30       31
chr20  2000  G    <INS>  .       INFO/IMPRECISE  missing_in_b  true     .
chr20  2000  G    <INS>  S1      FORMAT/GT       differs       1/1      0/1
chr20  3000  C    <DEL>  .       record          missing_in_b  .        .
```

| Column                       | Content                                                                    |
| ---------------------------- | -------------------------------------------------------------------------- |
| `chrom`, `pos`, `ref`, `alt` | record key, `.` for sample rows                                            |
| `sample`                     | sample name, `.` for site fields                                           |
| `field`                      | `ID`, `QUAL`, `FILTER`, `INFO/<key>`, `FORMAT/<key>`, `record` or `sample` |
| `kind`                       | `differs`, `missing_in_a` or `missing_in_b`                                |
| `value_a`, `value_b`         | values in each file, `.` when absent; INFO flags have the value `true`     |
