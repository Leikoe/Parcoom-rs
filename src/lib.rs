use std::ops;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
struct Parser<T> {
    run: Run<T>
}

type Run<T> = Arc<dyn Fn(ParserInput) -> (ParserInput, Result<T, String>)>;

#[derive(Debug, Clone)]
struct ParserInput {
    text: String,
    pos: usize
}

#[derive(Debug, Clone, PartialEq)]
struct ParserError {
    desc: String,
    pos: usize
}

fn input_sub (start: usize, len: usize, s: &ParserInput) -> ParserInput {
    ParserInput {
        text: s.text[start..start+len].to_string(),
        pos: s.pos+start
    }
}

fn fail<T> (e: String) -> Parser<T> {
    Parser {
        run: Arc::new(move |input| {
            let e = e.clone();
            (input, Err(e))
        })
    }
}

fn wrap<T: Clone + 'static> (x: T) -> Parser<T> {
    Parser {
        run: Arc::new(move |input| {
            let x = x.clone();
            (input, Ok(x))
        })
    }
}

fn map<A: 'static, B: 'static> (f: Box<dyn Fn(A) -> B>, p: Parser<A>) -> Parser<B> {
    Parser {
        run: Arc::new(move |input| {
            match (p.run)(input) {
                (input_, Ok(x)) => (input_, Ok(f(x))),
                (input_, Err(error)) => (input_, Err(error))
            }
        })
    }
}

fn parse_while (p: Box<dyn Fn(char) -> bool>) -> Parser<String> {
    Parser {
        run: Arc::new(move |input| {
            let n = input.text.len();
            let text = &input.text.as_bytes();
            let mut i = 0;
            while i < n && p(text[i] as char) {
                i += 1;
            }
            (input_sub(i, n-i, &input), Ok(input.text[0..i].to_string()))
        })
    }
}

fn bind<A: 'static, B: 'static> (f: Box<dyn Fn(A) -> Parser<B>>, p: Parser<A>) -> Parser<B> {
    Parser {
        run: Arc::new(move |input| {
            match (p.run)(input) {
                (input_, Ok(x)) => ((f(x)).run)(input_),
                (input_, Err(error)) => (input_, Err(error))
            }
        })
    }
}

fn prefix (prefix_str: &'static str) -> Parser<&str> {
    Parser {
        run: Arc::new(move |input| {
            let unexpected_prefix_error = format!("expected {}", prefix_str).to_string();

            let prefix_size = prefix_str.len();
            let input_size = input.text.len();

            let prefix_input = input_sub(0, prefix_size, &input);
            if prefix_input.text == prefix_str {
                let rest = input_sub(prefix_size, input_size - prefix_size, &input);
                (rest, Ok(prefix_str))
            } else {
                (input, Err(unexpected_prefix_error))
            }
        })
    }
}

fn optional<A: 'static> (p: Parser<A>) -> Parser<Option<A>> {
    Parser {
        run: Arc::new(move |input| {
            let (input_, result) = (p.run)(input);
            match result {
                Ok(x) => (input_, Ok(Some(x))),
                Err(_) => (input_, Ok(None))
            }
        })
    }
}

fn many_exact<A: 'static> (n: i32, p: Parser<A>) -> Parser<Vec<A>> {
    Parser {
        run: Arc::new(move |input| {
            let mut xs = Vec::new();
            let mut input_ = input;
            for _ in 0..n {
                let (input__, result) = (p.run)(input_);
                match result {
                    Ok(x) => {
                        xs.push(x);
                        input_ = input__;
                    },
                    Err(e) => {
                        return (input__, Err(e));
                    }
                }
            }
            (input_, Ok(xs))
        })
    }
}

fn many<A: 'static> (p: Parser<A>) -> Parser<Vec<A>> {
    Parser {
        run: Arc::new(move |input| {
            let mut xs = Vec::new();
            let mut input_ = input;
            loop {
                let (input__, result) = (p.run)(input_);
                input_ = input__;
                match result {
                    Ok(x) => {
                        xs.push(x);
                    },
                    Err(_) => {
                        break;
                    }
                }
            }
            (input_, Ok(xs))
        })
    }
}

fn any_char () -> Parser<char> {
    Parser {
        run: Arc::new(|input| {
            let n = input.text.len();
            if n >= 1 {
                (input_sub(1, n-1, &input), Ok(input.text.as_bytes()[0] as char))
            } else {
                let empty_input_error = format!("expected any char, got none (input.len() = {n}").to_string();

                (input, Err(empty_input_error))
            }
        })
    }
}

impl<B: 'static, A: 'static> ops::Shl<Parser<B>> for Parser<A> {
    type Output = Parser<A>;

    fn shl(self, p2: Parser<B>) -> Self::Output {
        Parser {
            run: Arc::new(move |input| {
                let (input_, result) = (self.run)(input);
                match result {
                    Ok(x) => {
                        let (input__, result_) = (p2.run)(input_);
                        match result_ {
                            Ok(_) => (input__, Ok(x)),
                            Err(e) => (input__, Err(e))
                        }
                    },
                    Err(e) => (input_, Err(e))
                }
            })
        }
    }
}

impl<B: 'static, A: 'static> ops::Shr<Parser<B>> for Parser<A> {
    type Output = Parser<B>;

    fn shr(self, p2: Parser<B>) -> Self::Output {
        Parser {
            run: Arc::new(move |input| {
                let (input_, result) = (self.run)(input);
                match result {
                    Ok(_) => {
                        let (input__, result_) = (p2.run)(input_);
                        match result_ {
                            Ok(x) => (input__, Ok(x)),
                            Err(e) => (input__, Err(e))
                        }
                    },
                    Err(e) => (input_, Err(e))
                }
            })
        }
    }
}

impl<B: 'static, A: 'static> ops::Add<Parser<B>> for Parser<A> {
    type Output = Parser<(A, B)>;

    fn add(self, p2: Parser<B>) -> Self::Output {
        Parser {
            run: Arc::new(move |input| {
                let (input_, result) = (self.run)(input);
                match result {
                    Ok(x) => {
                        let (input__, result_) = (p2.run)(input_);
                        match result_ {
                            Ok(x_) => (input__, Ok((x, x_))),
                            Err(e) => (input__, Err(e))
                        }
                    },
                    Err(e) => (input_, Err(e))
                }
            })
        }
    }
}

impl<A: 'static> ops::BitOr<Parser<A>> for Parser<A> {
    type Output = Parser<A>;

    fn bitor(self, p2: Parser<A>) -> Self::Output {
        Parser {
            run: Arc::new(move |input| {
                let (input_, result) = (self.run)(input.clone());
                match result {
                    Ok(x) => (input_, Ok(x)),
                    Err(_) => (p2.run)(input)
                }
            })
        }
    }
}

fn make_input (s: String) -> ParserInput {
    ParserInput {
        text: s,
        pos: 0
    }
}

fn run<A> (p: Parser<A>, input: String) -> Result<A, ParserError> {
    match (p.run)(make_input(input)) {
        (_, Ok(x)) => Ok(x),
        (input, Err(desc)) => Err(ParserError {
            desc,
            pos: input.pos
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_value_pair_parser_test() {
        let input = "key1 = value1".to_string();
        let wss = parse_while(Box::new(|x| x.is_whitespace()));

        let name_parser = parse_while(Box::new(|x| x.is_alphanumeric()));
        let entry_parser = (wss.clone() >> name_parser.clone() << wss.clone() << prefix("=")) + (wss.clone() >> name_parser.clone());

        let parsed = run(entry_parser, input);
        assert_eq!(parsed, Ok(("key1".to_string(), "value1".to_string())));
    }

    #[test]
    fn or_test() {
        let input = "111aaa".to_string();
        let parser = prefix("aaa") | prefix("111");

        let parsed = run(parser, input);
        assert_eq!(parsed, Ok("111"));
    }

    #[test]
    fn optional_test() {
        // test with working input
        let input = "111aaa".to_string();
        let parser = optional(prefix("111"));

        let parsed = run(parser, input);
        assert_eq!(parsed, Ok(Some("111")));

        // test with no valid input to parse
        let input = "aaa".to_string();
        let parser = optional(prefix("111"));

        let parsed = run(parser, input);
        assert_eq!(parsed, Ok(None));
    }

    #[test]
    fn any_char_test() {
        // test with working input
        let parser = any_char();
        let parsed = run(parser, "hello".to_string());
        assert_eq!(parsed, Ok('h'));

        // test with empty input, should fail
        let parser = any_char();
        let parsed = run(parser, "".to_string());

        assert_eq!(parsed, Err(ParserError{
            desc: format!("expected any char, got none (input.len() = {}", 0).to_string(),
            pos: 0
        }));
    }

    #[test]
    fn many_exact_test() {
        // test with input.len() = 3 (so parser succeeds)
        let input = "hel".to_string();
        let parser = many_exact(3, any_char());

        let parsed = run(parser, input);
        assert_eq!(parsed, Ok(vec!['h', 'e', 'l']));

        // test with input.len() < 3 (so parser fails)
        let input = "he".to_string();
        let parser = many_exact(3, any_char());

        let parsed = run(parser, input);
        assert_eq!(parsed, Err(ParserError{
            desc: format!("expected any char, got none (input.len() = {}", 0).to_string(),
            pos: 2
        }));
    }

    #[test]
    fn many_test() {
        // test with working input
        let input = "hello".to_string();
        let parser = many(any_char());

        let parsed = run(parser, input);
        assert_eq!(parsed, Ok(vec!['h', 'e', 'l', 'l', 'o']));
    }
}
