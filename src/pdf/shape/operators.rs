#[derive(Clone, Copy)]
pub(super) enum ColorRole {
    Stroke,
    Fill,
}

pub(super) fn format_g(value: f32) -> String {
    const PRECISION: i32 = 6;

    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_owned()
        } else {
            "inf".to_owned()
        };
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0".to_owned()
        } else {
            "0".to_owned()
        };
    }

    let sign = if value.is_sign_negative() { "-" } else { "" };
    let value = f64::from(value.abs());
    let scientific = format!("{:.*e}", (PRECISION - 1) as usize, value);
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return format!("{sign}{scientific}");
    };
    let Ok(exponent) = exponent.parse::<i32>() else {
        return format!("{sign}{scientific}");
    };

    if !(-4..PRECISION).contains(&exponent) {
        let mut mantissa = mantissa.to_owned();
        strip_trailing_fraction_zeros(&mut mantissa);
        format!(
            "{sign}{mantissa}e{}{abs_exponent:02}",
            if exponent < 0 { '-' } else { '+' },
            abs_exponent = exponent.abs()
        )
    } else {
        let digits_after_decimal = (PRECISION - exponent - 1).max(0) as usize;
        let mut fixed = format!("{sign}{value:.digits_after_decimal$}");
        strip_trailing_fraction_zeros(&mut fixed);
        fixed
    }
}

fn strip_trailing_fraction_zeros(value: &mut String) {
    if !value.contains('.') {
        return;
    }

    while value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
}

pub(super) fn color_code(components: &[f32], role: ColorRole) -> String {
    if !matches!(components.len(), 1 | 3 | 4)
        || !components
            .iter()
            .all(|component| component.is_finite() && (0.0..=1.0).contains(component))
    {
        return String::new();
    }

    let operator = match components.len() {
        1 => match role {
            ColorRole::Stroke => "G",
            ColorRole::Fill => "g",
        },
        3 => match role {
            ColorRole::Stroke => "RG",
            ColorRole::Fill => "rg",
        },
        4 => match role {
            ColorRole::Stroke => "K",
            ColorRole::Fill => "k",
        },
        _ => return String::new(),
    };
    let components = components
        .iter()
        .map(|component| format_g(*component))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{components} {operator}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_g_matches_python_percent_g_representatives() {
        let cases = [
            (0.0, "0"),
            (1.0, "1"),
            (-1.0, "-1"),
            (0.5, "0.5"),
            (0.1, "0.1"),
            (1.5e-6, "1.5e-06"),
            (123456.0, "123456"),
            (1234567.0, "1.23457e+06"),
            (0.0000001, "1e-07"),
            (-0.0, "-0"),
            (0.123_456_79, "0.123457"),
            (-0.123_456_79, "-0.123457"),
            (999999.9, "1e+06"),
            (0.00009999999, "0.0001"),
        ];

        for (input, expected) in cases {
            assert_eq!(format_g(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn format_g_is_deterministic_and_locale_independent() {
        const INPUTS: [f32; 4] = [0.5, 1234567.0, 1.5e-6, 0.123_456_79];
        let before: Vec<String> = INPUTS.into_iter().map(format_g).collect();

        for _ in 0..1000 {
            let repeated: Vec<String> = INPUTS.into_iter().map(format_g).collect();
            assert_eq!(repeated, before);
            assert!(repeated.iter().all(|value| !value.contains(',')));
        }

        std::env::set_var("LC_ALL", "de_DE.UTF-8");
        std::env::set_var("LC_NUMERIC", "de_DE.UTF-8");

        let under_locale: Vec<String> = INPUTS.into_iter().map(format_g).collect();
        assert_eq!(under_locale, before);
        assert!(under_locale.iter().all(|value| !value.contains(',')));

        let handles = (0..2).map(|_| {
            std::thread::spawn(|| {
                (0..1000)
                    .map(|_| INPUTS.into_iter().map(format_g).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            })
        });

        for handle in handles {
            for threaded in handle.join().unwrap() {
                assert_eq!(threaded, before);
                assert!(threaded.iter().all(|value| !value.contains(',')));
            }
        }
    }

    #[test]
    fn color_code_emits_role_specific_pdf_operators() {
        let cases = [
            (&[0.5][..], ColorRole::Stroke, "0.5 G\n"),
            (&[0.5][..], ColorRole::Fill, "0.5 g\n"),
            (&[1.0, 0.0, 0.0][..], ColorRole::Stroke, "1 0 0 RG\n"),
            (&[1.0, 0.0, 0.0][..], ColorRole::Fill, "1 0 0 rg\n"),
            (
                &[0.1, 0.2, 0.3, 0.4][..],
                ColorRole::Stroke,
                "0.1 0.2 0.3 0.4 K\n",
            ),
            (
                &[0.1, 0.2, 0.3, 0.4][..],
                ColorRole::Fill,
                "0.1 0.2 0.3 0.4 k\n",
            ),
        ];

        for (components, role, expected) in cases {
            assert_eq!(color_code(components, role), expected);
        }
    }

    #[test]
    fn color_code_rejects_invalid_components_with_empty_string() {
        let invalid_cases = [
            &[][..],
            &[0.5, 0.5][..],
            &[0.1, 0.1, 0.1, 0.1, 0.1][..],
            &[1.5, 0.0, 0.0][..],
            &[-0.1, 0.0, 0.0][..],
        ];

        for components in invalid_cases {
            assert_eq!(color_code(components, ColorRole::Fill), "");
            assert_eq!(color_code(components, ColorRole::Stroke), "");
        }
    }

    #[test]
    fn color_code_uses_format_g_for_components() {
        let expected = format!(
            "{} {} {} rg\n",
            format_g(0.123_456_79),
            format_g(0.0),
            format_g(0.0)
        );
        assert_eq!(
            color_code(&[0.123_456_79, 0.0, 0.0], ColorRole::Fill),
            expected
        );
    }
}
