use std::collections::HashMap;

/// How close two numbers must be to count as equal; with neither bound, numbers must be equal.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tolerance {
    abs: Option<f64>,
    rel: Option<f64>,
}

/// Parses `FIELD=SPEC`, where SPEC is `exact`, `abs:X`, `rel:X` or both bounds joined by a comma.
pub fn parse(arg: &str) -> Result<(String, Tolerance), String> {
    let (field, spec) = arg
        .split_once('=')
        .ok_or_else(|| format!("expected FIELD=SPEC, got '{arg}'"))?;
    if field.is_empty() {
        return Err(format!("missing field name in '{arg}'"));
    }
    let mut tolerance = Tolerance::default();
    if spec != "exact" {
        for bound in spec.split(',') {
            let (kind, value) = bound.split_once(':').ok_or_else(|| invalid(spec))?;
            let value: f64 = value.parse().map_err(|_| invalid(spec))?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "tolerance must be a non-negative number, got '{bound}'"
                ));
            }
            let slot = match kind {
                "abs" => &mut tolerance.abs,
                "rel" => &mut tolerance.rel,
                _ => return Err(invalid(spec)),
            };
            if slot.replace(value).is_some() {
                return Err(format!("'{kind}' given twice in '{spec}'"));
            }
        }
    }
    Ok((field.to_string(), tolerance))
}

fn invalid(spec: &str) -> String {
    format!("invalid tolerance '{spec}': expected exact, abs:X, rel:X or abs:X,rel:Y")
}

/// Checks that a tolerance targets a numeric field. Returns `false` when neither header declares it.
pub fn check_field(name: &str, declared: &HashMap<String, Vec<String>>) -> Result<bool, String> {
    if name == "QUAL" {
        return Ok(true);
    }
    if !(name.starts_with("INFO/") || name.starts_with("FORMAT/")) || name == "FORMAT/GT" {
        return Err(format!(
            "{name}: tolerances apply to QUAL and to numeric INFO/<key> and FORMAT/<key> fields"
        ));
    }
    let Some(types) = declared.get(name) else {
        return Ok(false);
    };
    match types.iter().find(|t| *t != "Integer" && *t != "Float") {
        Some(t) => Err(format!(
            "{name}: declared as {t}, tolerances apply to Integer and Float fields"
        )),
        None => Ok(true),
    }
}

impl Tolerance {
    /// Whether two values match within the tolerance; comma-separated lists match element by element.
    pub fn accepts(&self, a: &str, b: &str) -> bool {
        let xs: Vec<&str> = a.split(',').collect();
        let ys: Vec<&str> = b.split(',').collect();
        xs.len() == ys.len() && xs.iter().zip(&ys).all(|(x, y)| self.accepts_one(x, y))
    }

    /// Identical text always matches; otherwise both values must be numbers within a bound.
    fn accepts_one(&self, a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
        let (Ok(x), Ok(y)) = (a.parse::<f64>(), b.parse::<f64>()) else {
            return false;
        };
        let diff = (x - y).abs();
        diff == 0.0
            || self.abs.is_some_and(|t| diff <= t)
            || self.rel.is_some_and(|t| diff <= t * x.abs().max(y.abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance(spec: &str) -> Tolerance {
        parse(&format!("QUAL={spec}")).unwrap().1
    }

    #[test]
    fn parses_specs() {
        assert_eq!(tolerance("exact"), Tolerance::default());
        assert_eq!(
            tolerance("abs:0.5"),
            Tolerance {
                abs: Some(0.5),
                rel: None
            }
        );
        assert_eq!(
            tolerance("rel:1e-6,abs:2"),
            Tolerance {
                abs: Some(2.0),
                rel: Some(1e-6)
            }
        );
    }

    #[test]
    fn rejects_bad_specs() {
        for arg in [
            "QUAL",
            "=abs:1",
            "QUAL=",
            "QUAL=abs",
            "QUAL=abs:x",
            "QUAL=abs:-1",
            "QUAL=abs:inf",
            "QUAL=abs:1,abs:2",
            "QUAL=near:1",
        ] {
            assert!(parse(arg).is_err(), "{arg}");
        }
    }

    #[test]
    fn exact_compares_numbers() {
        let t = tolerance("exact");
        assert!(t.accepts("0.5", "0.50"));
        assert!(t.accepts("1e2", "100"));
        assert!(!t.accepts("0.5", "0.51"));
    }

    #[test]
    fn absolute_and_relative_bounds() {
        assert!(tolerance("abs:1").accepts("40", "41"));
        assert!(!tolerance("abs:1").accepts("40", "42"));
        assert!(tolerance("rel:0.1").accepts("100", "110"));
        assert!(tolerance("rel:0.1").accepts("110", "100"));
        assert!(!tolerance("rel:0.1").accepts("100", "112"));
        // Near zero only the absolute bound helps.
        assert!(!tolerance("rel:0.1").accepts("0", "0.001"));
        assert!(tolerance("rel:0.1,abs:0.01").accepts("0", "0.001"));
    }

    #[test]
    fn lists_match_element_by_element() {
        let t = tolerance("abs:1");
        assert!(t.accepts("10,12", "11,13"));
        assert!(!t.accepts("10,12", "10,14"));
        assert!(!t.accepts("5,5", "5,5,5"));
    }

    #[test]
    fn missing_and_text_values_need_identical_text() {
        let t = tolerance("abs:100");
        assert!(t.accepts(".", "."));
        assert!(!t.accepts(".", "30"));
        assert!(!t.accepts("NaN", "1"));
        assert!(t.accepts("NaN", "NaN"));
    }

    #[test]
    fn checks_field_types() {
        let declared = HashMap::from([
            ("INFO/AF".to_string(), vec!["Float".to_string()]),
            ("INFO/SOURCE".to_string(), vec!["String".to_string()]),
            (
                "FORMAT/DP".to_string(),
                vec!["Integer".to_string(), "String".to_string()],
            ),
        ]);
        assert_eq!(check_field("QUAL", &declared), Ok(true));
        assert_eq!(check_field("INFO/AF", &declared), Ok(true));
        assert_eq!(check_field("INFO/NOPE", &declared), Ok(false));
        for name in ["ID", "FILTER", "FORMAT/GT", "INFO/SOURCE", "FORMAT/DP"] {
            assert!(check_field(name, &declared).is_err(), "{name}");
        }
    }
}
