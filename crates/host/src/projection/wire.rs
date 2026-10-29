use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

pub(crate) fn trim_js(value: &str) -> &str {
    value.trim_matches(|c| {
        matches!(c,
        '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}')
    })
}

pub(crate) fn selector_id(number: f64) -> Option<i64> {
    (number.fract() == 0.0 && number.abs() <= 9_007_199_254_740_991.0).then_some(number as i64)
}

pub(crate) fn wire_number(value: &Value) -> Option<f64> {
    let number = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(value) => {
            let value = trim_js(value);
            if let Some(digits) = value
                .strip_prefix("0x")
                .or_else(|| value.strip_prefix("0X"))
            {
                radix_number(digits, 4)?
            } else if let Some(digits) = value
                .strip_prefix("0b")
                .or_else(|| value.strip_prefix("0B"))
            {
                radix_number(digits, 1)?
            } else if let Some(digits) = value
                .strip_prefix("0o")
                .or_else(|| value.strip_prefix("0O"))
            {
                radix_number(digits, 3)?
            } else {
                static DECIMAL: LazyLock<Regex> = LazyLock::new(|| {
                    Regex::new(r"^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$")
                        .unwrap()
                });
                if !DECIMAL.is_match(value) {
                    return None;
                }
                value.parse::<f64>().ok()?
            }
        }
        _ => return None,
    };
    number.is_finite().then_some(number)
}

pub(crate) fn wire_int(value: &Value) -> Option<f64> {
    let number = wire_number(value)?.trunc();
    (number.abs() <= 9_007_199_254_740_991.0).then_some(number)
}

pub(crate) fn wire_i64(value: &Value) -> Option<i64> {
    wire_int(value).map(|number| number as i64)
}

pub(crate) fn wire_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) if value.as_f64() == Some(1.0) => Some(true),
        Value::Number(value) if value.as_f64() == Some(0.0) => Some(false),
        Value::String(value) if value == "1" || value == "true" => Some(true),
        Value::String(value) if value == "0" || value == "false" => Some(false),
        _ => None,
    }
}

fn radix_number(digits: &str, bits_per_digit: u32) -> Option<f64> {
    if digits.is_empty() {
        return None;
    }
    let radix = 1_u32 << bits_per_digit;
    let mut significant_bits = 0_i32;
    let mut mantissa = 0_u64;
    let mut round_up = false;
    let mut sticky = false;
    for digit in digits.chars() {
        let digit = digit.to_digit(radix)?;
        for shift in (0..bits_per_digit).rev() {
            let bit = (digit >> shift) & 1;
            if significant_bits == 0 && bit == 0 {
                continue;
            }
            significant_bits = significant_bits.saturating_add(1);
            match significant_bits {
                1..=53 => mantissa = (mantissa << 1) | u64::from(bit),
                54 => round_up = bit != 0,
                _ => sticky |= bit != 0,
            }
        }
    }
    if significant_bits <= 53 {
        return Some(mantissa as f64);
    }
    if round_up && (sticky || mantissa & 1 == 1) {
        mantissa += 1;
    }
    let exponent = significant_bits - 53;
    if exponent > 1024 {
        return None;
    }
    Some(mantissa as f64 * 2.0_f64.powi(exponent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numeric_strings_follow_js_number_syntax_without_accepting_blank_values() {
        for (input, expected) in [
            ("\u{feff} 3.5 \u{00a0}", 3.5),
            ("+1e2", 100.0),
            (".5", 0.5),
            ("1.", 1.0),
            ("0x10", 16.0),
            ("0XfF", 255.0),
            ("0b101", 5.0),
            ("0o17", 15.0),
            ("0x20000000000003", 9007199254740996.0),
        ] {
            assert_eq!(wire_number(&json!(input)), Some(expected), "{input}");
        }
        for input in [
            "",
            " \t\u{feff}",
            "Infinity",
            "NaN",
            "inf",
            "1_000",
            "1 2",
            "-0x10",
            "+0b1",
            "0x",
            "0o8",
            "1e999",
            "\u{0085}3",
        ] {
            assert_eq!(wire_number(&json!(input)), None, "{input}");
        }
        for input in [json!(true), json!(null), json!([]), json!({})] {
            assert_eq!(wire_number(&input), None);
        }
    }

    #[test]
    fn wire_i64_truncates_toward_zero_and_requires_a_js_safe_integer() {
        assert_eq!(wire_i64(&json!(" -3.9 ")), Some(-3));
        assert_eq!(wire_i64(&json!(3.9)), Some(3));
        assert_eq!(wire_i64(&json!("0x1fffffffffffff")), Some(9007199254740991));
        assert_eq!(wire_i64(&json!("9007199254740992")), None);
        assert_eq!(wire_i64(&json!("-9007199254740992")), None);
        assert_eq!(wire_i64(&json!("-0")), Some(0));
    }

    #[test]
    fn wire_booleans_accept_only_the_exact_wire_spellings() {
        for value in [json!(true), json!(1), json!("1"), json!("true")] {
            assert_eq!(wire_bool(&value), Some(true));
        }
        for value in [json!(false), json!(0), json!("0"), json!("false")] {
            assert_eq!(wire_bool(&value), Some(false));
        }
        for value in [
            json!(2),
            json!("TRUE"),
            json!(" true"),
            json!("01"),
            json!(null),
            json!([]),
        ] {
            assert_eq!(wire_bool(&value), None);
        }
    }
}
