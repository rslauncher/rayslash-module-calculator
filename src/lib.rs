#[allow(warnings)]
mod bindings;

use bindings::exports::rayslash::module::provider::Guest;
use bindings::rayslash::module::types::{
    Action, Icon, ModuleError, QueryContext, QueryResponse, ResultItem,
};

struct Component;

impl Guest for Component {
    fn query(context: QueryContext) -> Result<QueryResponse, ModuleError> {
        let expression = context.query.trim();
        if expression.is_empty() || !calculation_hint(expression) {
            return Ok(QueryResponse {
                results: Vec::new(),
                exclusive: false,
            });
        }
        let outcome = if expression.contains('=') {
            solve_linear(expression)
        } else {
            evaluate(expression)
        };
        let (title, action) = match outcome {
            Ok(result) => (result.clone(), Action::CopyText(result)),
            Err(message) => (message.clone(), Action::ShowMessage(message)),
        };
        Ok(QueryResponse {
            results: vec![ResultItem {
                id: format!("calculator:{}", expression.to_ascii_lowercase()),
                title,
                subtitle: format!("Calculate: {expression}"),
                icon: Icon::Text("=".into()),
                score: None,
                action,
            }],
            exclusive: false,
        })
    }
}

fn evaluate(expression: &str) -> Result<String, String> {
    let mut context = fend_core::Context::new();
    context.set_random_u32_fn(|| 4);
    let result = fend_core::evaluate(expression, &mut context)?;
    if result.output_is_empty() {
        return Err("Expression has no numeric result.".into());
    }
    let text = result.get_main_result().trim().to_owned();
    (!text.is_empty())
        .then_some(text)
        .ok_or_else(|| "Expression has no result.".into())
}

fn solve_linear(expression: &str) -> Result<String, String> {
    let (left, right) = expression
        .split_once('=')
        .ok_or("Equation must contain one equals sign.")?;
    if right.contains('=') || !contains_variable(left) && !contains_variable(right) {
        return Err("Equation must contain one variable named x.".into());
    }
    let value = |x: f64| -> Result<f64, String> {
        let combined = format!("({})-({})", replace_x(left, x), replace_x(right, x));
        evaluate(&combined)?
            .replace(',', "")
            .parse::<f64>()
            .map_err(|_| "Equation did not produce a plain number.".into())
    };
    let y0 = value(0.0)?;
    let y1 = value(1.0)?;
    let y2 = value(2.0)?;
    let slope = y1 - y0;
    if (y2 - (y0 + 2.0 * slope)).abs() > 1e-8 {
        return Err("Only linear equations in x are supported.".into());
    }
    if slope.abs() < 1e-12 {
        return Err(if y0.abs() < 1e-12 {
            "Equation has infinitely many solutions."
        } else {
            "Equation has no solution."
        }
        .into());
    }
    Ok(format!("x = {}", format_number(-y0 / slope)))
}

fn replace_x(expression: &str, value: f64) -> String {
    let chars = expression.chars().collect::<Vec<_>>();
    chars
        .iter()
        .enumerate()
        .map(|(index, ch)| {
            if *ch == 'x'
                && index
                    .checked_sub(1)
                    .and_then(|i| chars.get(i))
                    .is_none_or(|ch| !ch.is_ascii_alphabetic())
                && chars
                    .get(index + 1)
                    .is_none_or(|ch| !ch.is_ascii_alphabetic())
            {
                format!("({value})")
            } else {
                ch.to_string()
            }
        })
        .collect()
}

fn contains_variable(expression: &str) -> bool {
    replace_x(expression, 0.0) != expression
}
fn calculation_hint(value: &str) -> bool {
    let has_digit = value.chars().any(|ch| ch.is_ascii_digit());
    has_digit
        && value
            .chars()
            .any(|ch| matches!(ch, '+' | '-' | '*' | '/' | '^' | '=' | '(' | ')' | '.'))
        || value.contains("sqrt(")
        || value.contains("sin(")
        || value.contains("cos(")
}
fn format_number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-10 {
        return format!("{:.0}", value);
    }
    format!("{value:.12}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

bindings::export!(Component with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evaluates_expressions() {
        assert_eq!(evaluate("2 * (3 + 4)").unwrap(), "14");
    }
    #[test]
    fn solves_linear_equations() {
        assert_eq!(solve_linear("2x + 4 = 10").unwrap(), "x = 3");
    }
    #[test]
    fn ignores_plain_queries() {
        assert!(!calculation_hint("firefox"));
    }
}
