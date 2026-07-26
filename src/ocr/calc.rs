//! 计算题验证码求值:把 OCR 识别出的算术式文本(可能带中文数字 / 中文运算符 / 全角 / 干扰字)
//! 解析成表达式并求值。**纯函数、后端无关、零依赖**,与 OCR 识别解耦(便于离线单测)。
//!
//! 支持:`+ - * /` 与中文「加 减 乘 除」、全角运算符(`＋ － × ÷ ／`)、阿拉伯 / 全角 / 中文数字
//! (`零〇一二三四五六七八九十两` 及大写 `壹…拾`)、去掉「= ? 等于 =?」等尾巴、忽略其它干扰字符,
//! 运算符优先级(`* /` 高于 `+ -`)与一元负号。要求至少两个操作数(否则视作非计算题,返回 `None`)。

/// 从一段含算术式的文本解出并求值。返回 `None` 表示无法解析为(含运算的)算术式。
///
/// ```
/// # use drission::ocr::eval_calc;
/// assert_eq!(eval_calc("3+5=?"), Some(8.0));
/// assert_eq!(eval_calc("三加五等于"), Some(8.0));
/// assert_eq!(eval_calc("2+3*4"), Some(14.0));
/// assert_eq!(eval_calc("abc"), None);
/// ```
pub fn eval_calc(text: &str) -> Option<f64> {
    let clean = normalize(text);
    let toks = tokenize(&clean)?;
    let num_count = toks.iter().filter(|t| matches!(t, Tok::Num(_))).count();
    if num_count < 2 {
        return None; // 计算题至少两个操作数;单个数字视作非计算题。
    }
    let mut p = Parser { toks, pos: 0 };
    let v = p.expr()?;
    if p.pos == p.toks.len() { Some(v) } else { None }
}

/// 把求值结果格式化成填空用字符串:整数不带小数点(`8.0` → `"8"`),小数去尾零。
pub fn calc_answer_string(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Op(char),
}

/// 归一化:全角 / 中文运算符→ASCII,全角 / 中文数字→阿拉伯,其它字符→空格分隔。
fn normalize(text: &str) -> String {
    // Pass A:运算符与全角数字直接映射,中文数字暂留原字符,其它变空格。
    let mut work: Vec<char> = Vec::with_capacity(text.len());
    for ch in text.chars() {
        let mapped = match ch {
            '0'..='9' | '+' | '-' | '*' | '/' | '.' | ' ' => ch,
            '０'..='９' => char::from_digit(ch as u32 - '０' as u32, 10).unwrap_or(' '),
            '＋' | '加' => '+',
            '－' | '—' | '﹣' | 'ー' | '减' | '減' => '-',
            '×' | '＊' | '✕' | '⨯' | '乘' => '*',
            '÷' | '／' | '➗' | '除' => '/',
            _ if is_cjk_numeral(ch) => ch,
            _ => ' ',
        };
        work.push(mapped);
    }
    // Pass B:把连续的中文数字段转成阿拉伯数字串。
    let mut out = String::with_capacity(work.len());
    let mut i = 0;
    while i < work.len() {
        if is_cjk_numeral(work[i]) {
            let start = i;
            while i < work.len() && is_cjk_numeral(work[i]) {
                i += 1;
            }
            if let Some(n) = cjk_run_to_number(&work[start..i]) {
                out.push_str(&n.to_string());
            } else {
                out.push(' ');
            }
        } else {
            out.push(work[i]);
            i += 1;
        }
    }
    out
}

fn is_cjk_numeral(c: char) -> bool {
    matches!(
        c,
        '零' | '〇'
            | '一'
            | '二'
            | '两'
            | '三'
            | '四'
            | '五'
            | '六'
            | '七'
            | '八'
            | '九'
            | '十'
            | '壹'
            | '贰'
            | '叁'
            | '肆'
            | '伍'
            | '陆'
            | '柒'
            | '捌'
            | '玖'
            | '拾'
    )
}

fn cjk_digit(c: char) -> Option<i64> {
    Some(match c {
        '零' | '〇' => 0,
        '一' | '壹' => 1,
        '二' | '两' | '贰' => 2,
        '三' | '叁' => 3,
        '四' | '肆' => 4,
        '五' | '伍' => 5,
        '六' | '陆' => 6,
        '七' | '柒' => 7,
        '八' | '捌' => 8,
        '九' | '玖' => 9,
        _ => return None,
    })
}

/// 把一段连续中文数字(可含 `十/拾`)转成整数。支持 `十=10`、`十五=15`、`二十=20`、`二十三=23`。
fn cjk_run_to_number(run: &[char]) -> Option<i64> {
    if let Some(pos) = run.iter().position(|&c| c == '十' || c == '拾') {
        let left = &run[..pos];
        let right = &run[pos + 1..];
        let tens = if left.is_empty() {
            1
        } else {
            cjk_digit(*left.last()?)?
        };
        let units = if right.is_empty() {
            0
        } else {
            cjk_digit(*right.first()?)?
        };
        Some(tens * 10 + units)
    } else {
        let mut acc = 0i64;
        let mut any = false;
        for &c in run {
            acc = acc * 10 + cjk_digit(c)?;
            any = true;
        }
        if any { Some(acc) } else { None }
    }
}

fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = s.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num: f64 = chars[start..i].iter().collect::<String>().parse().ok()?;
            toks.push(Tok::Num(num));
        } else if matches!(c, '+' | '-' | '*' | '/') {
            toks.push(Tok::Op(c));
            i += 1;
        } else {
            return None;
        }
    }
    Some(toks)
}

/// 递归下降:`expr := term (('+'|'-') term)*`,`term := factor (('*'|'/') factor)*`,
/// `factor := number | ('+'|'-') factor`(一元号)。
struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '-'))) = self.peek() {
            let c = *c;
            self.pos += 1;
            let r = self.term()?;
            v = if c == '+' { v + r } else { v - r };
        }
        Some(v)
    }

    fn term(&mut self) -> Option<f64> {
        let mut v = self.factor()?;
        while let Some(Tok::Op(c @ ('*' | '/'))) = self.peek() {
            let c = *c;
            self.pos += 1;
            let r = self.factor()?;
            if c == '/' {
                if r == 0.0 {
                    return None;
                }
                v /= r;
            } else {
                v *= r;
            }
        }
        Some(v)
    }

    fn factor(&mut self) -> Option<f64> {
        match self.peek()? {
            Tok::Op('-') => {
                self.pos += 1;
                Some(-self.factor()?)
            }
            Tok::Op('+') => {
                self.pos += 1;
                self.factor()
            }
            Tok::Num(n) => {
                let n = *n;
                self.pos += 1;
                Some(n)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_expressions() {
        assert_eq!(eval_calc("3+5"), Some(8.0));
        assert_eq!(eval_calc("3+5=?"), Some(8.0));
        assert_eq!(eval_calc("12-4="), Some(8.0));
        assert_eq!(eval_calc("3*2"), Some(6.0));
        assert_eq!(eval_calc("6/2=?"), Some(3.0));
        assert_eq!(eval_calc("9-3-2"), Some(4.0)); // 左结合
        assert_eq!(eval_calc("2+3*4"), Some(14.0)); // 优先级
        assert_eq!(eval_calc("-3+5"), Some(2.0)); // 一元负号
    }

    #[test]
    fn fullwidth_and_symbol_operators() {
        assert_eq!(eval_calc("３＋５＝？"), Some(8.0));
        assert_eq!(eval_calc("7×8"), Some(56.0));
        assert_eq!(eval_calc("8÷4=?"), Some(2.0));
    }

    #[test]
    fn chinese_numerals_and_operators() {
        assert_eq!(eval_calc("三加五等于"), Some(8.0));
        assert_eq!(eval_calc("十二减四"), Some(8.0));
        assert_eq!(eval_calc("二十三加七"), Some(30.0));
        assert_eq!(eval_calc("十五除以三"), Some(5.0));
        assert_eq!(eval_calc("两乘以九"), Some(18.0));
    }

    #[test]
    fn noise_is_ignored() {
        assert_eq!(eval_calc("请计算 3 + 5 = ? 的结果"), Some(8.0));
        assert_eq!(eval_calc("答案:12*2"), Some(24.0));
    }

    #[test]
    fn rejects_non_calc() {
        assert_eq!(eval_calc(""), None);
        assert_eq!(eval_calc("abc"), None);
        assert_eq!(eval_calc("5"), None); // 单个数字不是计算题
        assert_eq!(eval_calc("1/0"), None); // 除零
        assert_eq!(eval_calc("3 5"), None); // 两数无运算符
    }

    #[test]
    fn answer_formatting() {
        assert_eq!(calc_answer_string(8.0), "8");
        assert_eq!(calc_answer_string(-3.0), "-3");
        assert_eq!(calc_answer_string(2.5), "2.5");
    }
}
