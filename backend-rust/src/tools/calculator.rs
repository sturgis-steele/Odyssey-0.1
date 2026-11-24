use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use serde_json::{Number, Value};
use std::{str::FromStr, sync::LazyLock};

const SYNTAX: &str = "This expression cannot be calculated. Check its syntax and operands.";
const DOMAIN: &str = "Use numbers, arithmetic operators, parentheses or sqrt(x).";
const RANGE: &str = "The result exceeds the supported real-number range.";
static LIMIT: LazyLock<BigInt> = LazyLock::new(|| BigInt::from(10u8).pow(100));
type Calc<T> = Result<T, &'static str>;

enum Numeric {
    Int(BigInt),
    Float(f64),
}
impl Numeric {
    fn check(self) -> Calc<Self> {
        match &self {
            Self::Int(i) if i.abs() > *LIMIT => Err(RANGE),
            Self::Float(f) if !f.is_finite() || f.abs() >= 1e100 => Err(RANGE),
            _ => Ok(self),
        }
    }
    fn float(&self) -> f64 {
        match self {
            Self::Int(i) => i.to_f64().expect("bounded integer"),
            Self::Float(v) => *v,
        }
    }
    fn neg(self) -> Self {
        match self {
            Self::Int(n) => Self::Int(-n),
            Self::Float(n) => Self::Float(-n),
        }
    }
    fn apply(self, op: &str, right: Self) -> Calc<Self> {
        let b = right.float();
        if matches!(op, "/" | "//" | "%") && b == 0.0 {
            return Err(SYNTAX);
        }
        if op == "**" && b.abs() > 100.0 {
            return Err("Use an exponent between -100 and 100.");
        }
        if let (Self::Int(x), Self::Int(y)) = (&self, &right) {
            let int = match op {
                "+" => Some(x + y),
                "-" => Some(x - y),
                "*" => Some(x * y),
                "//" | "%" => {
                    let r = x % y;
                    let adjust = !r.is_zero() && r.is_negative() != y.is_negative();
                    Some(if op == "%" {
                        if adjust { r + y } else { r }
                    } else if adjust {
                        x / y - 1
                    } else {
                        x / y
                    })
                }
                "**" if !y.is_negative() => Some(x.pow(y.to_u32().ok_or(RANGE)?)),
                _ => None,
            };
            if let Some(n) = int {
                return Self::Int(n).check();
            }
        }
        let a = self.float();
        let v = match op {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" => a / b,
            "//" | "%" => {
                let mut rem = a % b;
                let mut div = (a - rem) / b;
                if rem != 0.0 && rem.is_sign_negative() != b.is_sign_negative() {
                    rem += b;
                    div -= 1.0;
                }
                if op == "%" {
                    if rem == 0.0 { 0.0f64.copysign(b) } else { rem }
                } else if div == 0.0 {
                    0.0f64.copysign(a / b)
                } else {
                    let floor = div.floor();
                    if div - floor > 0.5 {
                        floor + 1.0
                    } else {
                        floor
                    }
                }
            }
            "**" if a == 0.0 && b < 0.0 => return Err(SYNTAX),
            "**" => a.powf(b),
            _ => return Err(DOMAIN),
        };
        Self::Float(v).check()
    }
}
struct Parser<'a> {
    tail: &'a str,
    nodes: usize,
}
impl Parser<'_> {
    fn nodes(&mut self, n: usize) -> Calc<()> {
        self.nodes += n;
        if self.nodes > 64 {
            Err("Use a shorter expression.")
        } else {
            Ok(())
        }
    }
    fn eat(&mut self, token: &str) -> bool {
        self.tail = self.tail.trim_start_matches([' ', '\t', '\x0c']);
        if let Some(rest) = self.tail.strip_prefix(token) {
            self.tail = rest;
            true
        } else {
            false
        }
    }
    fn expr(&mut self, min: u8) -> Calc<Numeric> {
        let mut left = if self.eat("-") {
            self.nodes(2)?;
            self.expr(3)?.neg()
        } else if self.eat("+") {
            self.nodes(2)?;
            self.expr(3)?
        } else if self.eat("(") {
            let v = self.expr(0)?;
            if !self.eat(")") {
                return Err(SYNTAX);
            }
            v
        } else if self.eat("sqrt") {
            self.nodes(3)?;
            if !self.eat("(") {
                return Err(DOMAIN);
            }
            let v = self.expr(0)?.float();
            if !self.eat(")") {
                return Err(SYNTAX);
            }
            if v < 0.0 {
                return Err("math domain error");
            }
            Numeric::Float(v.sqrt())
        } else {
            self.nodes(1)?;
            self.number()?
        };
        loop {
            self.tail = self.tail.trim_start_matches([' ', '\t', '\x0c']);
            let Some(&(op, precedence)) = [
                ("**", 4),
                ("//", 2),
                ("*", 2),
                ("/", 2),
                ("%", 2),
                ("+", 1),
                ("-", 1),
            ]
            .iter()
            .find(|(op, _)| self.tail.starts_with(op)) else {
                break;
            };
            if precedence < min {
                break;
            }
            self.tail = &self.tail[op.len()..];
            self.nodes(2)?;
            let right = self.expr(if op == "**" { 3 } else { precedence + 1 })?;
            left = left.apply(op, right)?;
        }
        Ok(left)
    }
    fn number(&mut self) -> Calc<Numeric> {
        let raw = self.tail;
        let bytes = raw.as_bytes();
        let radix = match bytes.get(..2) {
            Some(b"0x" | b"0X") => 16,
            Some(b"0o" | b"0O") => 8,
            Some(b"0b" | b"0B") => 2,
            _ => 10,
        };
        if radix != 10 {
            let n = raw[2..]
                .bytes()
                .take_while(|b| b.is_ascii_hexdigit() || *b == b'_')
                .count();
            let digits = raw[2..2 + n].strip_prefix('_').unwrap_or(&raw[2..2 + n]);
            if digits.is_empty()
                || digits.starts_with('_')
                || digits.ends_with('_')
                || digits.contains("__")
            {
                return Err(SYNTAX);
            }
            self.tail = &raw[2 + n..];
            return Numeric::Int(
                BigInt::parse_bytes(digits.replace('_', "").as_bytes(), radix).ok_or(SYNTAX)?,
            )
            .check();
        }
        let mut i = 0;
        while let Some(&b) = bytes.get(i) {
            if b.is_ascii_digit()
                || matches!(b, b'.' | b'_' | b'e' | b'E')
                || (matches!(b, b'+' | b'-') && i > 0 && matches!(bytes[i - 1], b'e' | b'E'))
            {
                i += 1;
            } else {
                break;
            }
        }
        if i == 0 {
            return Err(DOMAIN);
        }
        let token = &bytes[..i];
        for (pos, b) in token.iter().enumerate() {
            if *b == b'_'
                && (pos == 0
                    || !token[pos - 1].is_ascii_digit()
                    || !token.get(pos + 1).is_some_and(u8::is_ascii_digit))
            {
                return Err(SYNTAX);
            }
        }
        let cleaned = raw[..i].replace('_', "");
        self.tail = &raw[i..];
        if cleaned.contains(['.', 'e', 'E']) {
            Numeric::Float(cleaned.parse().map_err(|_| SYNTAX)?).check()
        } else {
            if cleaned.starts_with('0') && cleaned.bytes().any(|b| b != b'0') {
                return Err(SYNTAX);
            }
            Numeric::Int(BigInt::from_str(&cleaned).map_err(|_| SYNTAX)?).check()
        }
    }
}
// Python arithmetic permits comments and continuation inside parentheses.
fn logical_line(expression: &str) -> Calc<String> {
    let mut result = String::new();
    let mut depth = 0i16;
    let mut complete = false;
    for line in expression.trim().lines() {
        let code = line.split('#').next().unwrap_or("").trim_end();
        if code.is_empty() {
            continue;
        }
        if complete {
            return Err(SYNTAX);
        }
        let continued = code.strip_suffix('\\');
        let code = continued.unwrap_or(code);
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(code);
        for c in code.bytes() {
            match c {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => (),
            }
            if depth < 0 {
                return Err(SYNTAX);
            }
        }
        complete = depth == 0 && continued.is_none();
    }
    if !complete {
        return Err(SYNTAX);
    }
    Ok(result)
}
pub(super) fn calculate(expression: &str) -> Calc<Value> {
    let expression = logical_line(expression)?;
    let mut p = Parser {
        tail: expression.trim(),
        nodes: 1,
    };
    let value = p.expr(0)?;
    if !p.tail.is_empty() {
        return Err(SYNTAX);
    }
    Ok(Value::Number(match value {
        Numeric::Int(i) => i.to_string().parse::<Number>().map_err(|_| RANGE)?,
        Numeric::Float(f) => Number::from_f64(f).ok_or(RANGE)?,
    }))
}
