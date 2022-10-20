use std::ops;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
struct Parser<T> {
    run: Run<T>
}

type Run<T> = Arc<dyn Fn(ParserInput) -> (ParserInput, Result<T, ParserError>)>;

#[derive(Debug)]
struct ParserInput {
    text: String,
    pos: usize
}

#[derive(Debug, Clone)]
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

fn fail<T> (e: ParserError) -> Parser<T> {
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
            let unexpected_prefix_error = ParserError {
                desc: format!("expected {}", prefix_str).to_string(),
                pos: 0
            };

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

fn make_input (s: String) -> ParserInput {
    ParserInput {
        text: s,
        pos: 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let input = make_input("key1 = value1".to_string());
        let wss = parse_while(Box::new(|x| x.is_whitespace()));

        let name_parser = parse_while(Box::new(|x| x.is_alphanumeric()));
        let entry_parser = (wss.clone() >> name_parser.clone() << wss.clone() << prefix("=")) + (wss.clone() >> name_parser.clone());

        let output = (entry_parser.run)(input);
        dbg!(output);
    }
}
