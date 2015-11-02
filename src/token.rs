use std::collections::LinkedList;

// Parser template file into a series of tokens

// All tokens appears in mustache
#[derive(Debug, PartialEq)]
pub enum Token {
    // regex: .*
    Text(String),
    // regex: LM
    LMustache,
    // regex: RM
    RMustache,

    // Following are enabled when between LM and RM.

    // regex: &
    UnescapeTag,
    // regex: #
    Pound,
    // regex: /
    Slash,
    // regex: ^
    Hat,
    // Regex: (\w|\d)+
    Id(String)
}

enum State {
    Normal,
    SyntaxToken
}

pub struct Tokenlizer<'a, T: 'a> {
    reader : &'a mut T,
    // We can push back some char if read more char than we want.
    buf    : LinkedList<char>,
    // Whether or not current position is between lm and rm
    state  : State,
    lm     : String,
    rm     : String
}

fn is_id(ch : char) -> bool {
    // Match [a-zA-Z0-9_]
    ch.is_digit(36) || ch == '_'
}


impl<'a, T : Iterator<Item = char>> Tokenlizer<'a, T> {

    fn new(lm: &str, rm: &str, reader: &'a mut T) -> Tokenlizer<'a, T> {
        Tokenlizer {
            reader : reader,
            buf    : LinkedList::new(),
            state  : State::Normal,
            lm     : lm.to_string(),
            rm     : rm.to_string()
        }
    }

    // Char stream operations
    fn read(&mut self) -> Option<char> {
        self
            .buf.pop_front()
            .or_else(|| self.reader.next())
    }

    fn read_str(&mut self, size : usize) -> Option<String> {
        let mut buf = String::new();
        for _ in 0..size {
            match self.read() {
                Some(c) => buf.push(c),
                None => {
                    self.push_back_str(&buf);
                    return None;
                }
            }
        }
        Some(buf)
    }

    fn read_until(&mut self, until : &str) -> Option<String> {
        let mut buf = String::new();
        // Read until end of file or encounter "until"
        while let Some(c) = self.read() {
            buf.push(c);
            if buf.ends_with(until) {
                self.push_back_str(until);
                let len = buf.len() - until.len();
                buf.truncate(len);
                break;
            }
        }

        if buf.len() > 0 {
            Some(buf)
        } else {
            None
        }
    }

    fn read_while<F>(&mut self, pred : F) -> Option<String> where 
        F: Fn(char) -> bool
    {
        let mut buf = String::new();
        while let Some(c) = self.read() {
            if pred(c) {
                buf.push(c);
            } else {
                self.push_back_char(c);
                break;
            }
        }

        if buf.len() > 0 {
            Some(buf)
        } else {
            None
        }
    }

    fn push_back_char(&mut self, ch: char) {
        self.buf.push_front(ch);
    }

    fn push_back_str(&mut self, st: &str) {
        let mut buf : Vec<_> = st.chars().collect();
        buf.reverse();

        for ch in buf {
            self.push_back_char(ch);
        }
    }

    fn starts_with(&mut self, st: &str) -> bool {
        if let Some(temp) = self.read_str(st.len()) {
            if &temp == st {
                return true;
            } else {
                self.push_back_str(&temp);
            }
        }
        false
    }

    fn skip_ws(&mut self) {
        loop {
            match self.read() {
                Some(c) if c == ' ' => {}
                Some(c) => {
                    self.push_back_char(c);
                    break;
                }
                _ => {break;}
            }
        }
    }

    fn read_token(&mut self) -> Option<Token> {
        macro_rules! token_rules {
            ($($e:expr => $a:expr),*) => {{
                $(if self.starts_with($e) { return Some($a); })* 
            }}
        }

        let lm = self.lm.clone();
        let rm = self.rm.clone();

        match self.state {
            State::Normal => {
                if self.starts_with(&lm) {
                    self.state = State::SyntaxToken;
                    Some(Token::LMustache)
                } else {
                    self.read_until(&lm)
                        .map(Token::Text)
                }
            }

            State::SyntaxToken => {
                self.skip_ws();

                token_rules! {
                    "#" => Token::Pound,
                    "&" => Token::UnescapeTag,
                    "/" => Token::Slash,
                    "^" => Token::Hat,
                    &rm => {
                        self.state = State::Normal;
                        Token::RMustache
                    }
                }

                self.read_while(is_id)
                    .map(Token::Id)
            }
        }
    }
}

impl <'a, T> Iterator for Tokenlizer<'a, T> 
where T: 'a + Iterator<Item = char> 
{
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        self.read_token()
    }
}

#[cfg(test)]
mod test {
    use super::Tokenlizer;
    use super::Token;
    use super::Token::*;

    #[test]
    fn test_push_back() {
        let mut stream = "abc".chars();
        let mut tokenlizer = Tokenlizer::new("{{", "}}", &mut stream);

        assert_eq!(tokenlizer.read(), Some('a'));
        tokenlizer.push_back_char('a');
        assert_eq!(tokenlizer.read(), Some('a'));

        tokenlizer.push_back_char('a');
        assert_eq!(tokenlizer.read_str(3), Some("abc".to_string()));

        tokenlizer.push_back_str("abc");
        assert_eq!(tokenlizer.read_str(3), Some("abc".to_string()));
    }

    fn test_parser(source: &str, expected : Vec<Token>) {
        let mut stream = source.chars();
        let tokenlizer = Tokenlizer::new("{{", "}}", &mut stream);
        let result : Vec<_> = tokenlizer.collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_mustache() {
        test_parser("abc{{bcd}}", vec![Text("abc".to_string()), LMustache, Id("bcd".to_string()), RMustache]);
    }

    #[test]
    fn test_parse_other_ops() {
        test_parser("abc{{^ # abc}}bcd", vec![Text("abc".to_string()), LMustache, Hat, Pound, Id("abc".to_string()), RMustache, Text("bcd".to_string())]);
    }
}
