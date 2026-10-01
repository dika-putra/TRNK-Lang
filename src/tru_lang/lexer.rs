//! TRU Language Lexer (Tokenizer)
//! Converts .tru source code into a flat stream of tokens.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Literals ──────────────────────────────────────────────────
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,

    // ── Identifiers / Keywords ────────────────────────────────────
    Ident(String),

    // keywords
    Let,
    Mut,
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Struct,
    Impl,
    Enum,
    Match,
    Use,
    Mod,
    Pub,
    Self_,
    True,
    False,
    Type,
    Trait,
    Import,
    Print,
    Println,

    // ── Operators ─────────────────────────────────────────────────
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Eq,          // =
    EqEq,        // ==
    BangEq,      // !=
    Lt,          // <
    LtEq,        // <=
    Gt,          // >
    GtEq,        // >=
    And,         // &&
    Or,          // ||
    Bang,        // !
    Ampersand,   // &
    Pipe,        // |
    Caret,       // ^
    Tilde,       // ~
    Shl,         // <<
    Shr,         // >>
    Arrow,       // ->
    FatArrow,    // =>
    Dot,         // .
    DotDot,      // ..
    ColonColon,  // ::
    PlusEq,      // +=
    MinusEq,     // -=
    StarEq,      // *=
    SlashEq,     // /=

    // ── Delimiters ────────────────────────────────────────────────
    LParen,      // (
    RParen,      // )
    LBrace,      // {
    RBrace,      // }
    LBracket,    // [
    RBracket,    // ]
    Comma,       // ,
    Semicolon,   // ;
    Colon,       // :
    Hash,        // #
    At,          // @

    // ── Meta ──────────────────────────────────────────────────────
    Eof,
    Newline,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Self { kind, span: Span { line, col } }
    }
}

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            src: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof { break; }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.src.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.src.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' { self.line += 1; self.col = 1; }
            else { self.col += 1; }
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => { self.advance(); }
                Some('\n') => { self.advance(); }
                Some('/') if self.peek2() == Some('/') => {
                    // line comment
                    while self.peek().map(|c| c != '\n').unwrap_or(false) {
                        self.advance();
                    }
                }
                Some('/') if self.peek2() == Some('*') => {
                    self.advance(); self.advance(); // consume /*
                    loop {
                        match self.peek() {
                            None => break,
                            Some('*') if self.peek2() == Some('/') => {
                                self.advance(); self.advance(); break;
                            }
                            _ => { self.advance(); }
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self) -> Result<String, LexError> {
        // opening `"` already consumed
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(LexError::UnterminatedString(self.line)),
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n')  => s.push('\n'),
                    Some('t')  => s.push('\t'),
                    Some('r')  => s.push('\r'),
                    Some('\\') => s.push('\\'),
                    Some('"')  => s.push('"'),
                    Some(c)    => { s.push('\\'); s.push(c); }
                    None => return Err(LexError::UnterminatedString(self.line)),
                },
                Some(c) => s.push(c),
            }
        }
        Ok(s)
    }

    fn read_number(&mut self, first: char) -> TokenKind {
        let mut num = String::new();
        num.push(first);
        let mut is_float = false;

        loop {
            match self.peek() {
                Some(c) if c.is_ascii_digit() => { num.push(c); self.advance(); }
                Some('.') if !is_float && self.peek2().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                    is_float = true;
                    num.push('.'); self.advance();
                }
                Some('_') => { self.advance(); } // allow 1_000_000
                _ => break,
            }
        }

        if is_float {
            TokenKind::Float(num.parse().unwrap_or(0.0))
        } else {
            TokenKind::Int(num.parse().unwrap_or(0))
        }
    }

    fn read_ident_or_keyword(&mut self, first: char) -> TokenKind {
        let mut word = String::new();
        word.push(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                word.push(c); self.advance();
            } else { break; }
        }
        match word.as_str() {
            "let"      => TokenKind::Let,
            "mut"      => TokenKind::Mut,
            "fn"       => TokenKind::Fn,
            "return"   => TokenKind::Return,
            "if"       => TokenKind::If,
            "else"     => TokenKind::Else,
            "while"    => TokenKind::While,
            "for"      => TokenKind::For,
            "in"       => TokenKind::In,
            "break"    => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "struct"   => TokenKind::Struct,
            "impl"     => TokenKind::Impl,
            "enum"     => TokenKind::Enum,
            "match"    => TokenKind::Match,
            "use"      => TokenKind::Use,
            "mod"      => TokenKind::Mod,
            "pub"      => TokenKind::Pub,
            "self"     => TokenKind::Self_,
            "true"     => TokenKind::True,
            "false"    => TokenKind::False,
            "nil"      => TokenKind::Nil,
            "type"     => TokenKind::Type,
            "trait"    => TokenKind::Trait,
            "import"   => TokenKind::Import,
            "print"    => TokenKind::Print,
            "println"  => TokenKind::Println,
            _          => TokenKind::Ident(word),
        }
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();
        let line = self.line;
        let col  = self.col;

        let ch = match self.advance() {
            None => return Ok(Token::new(TokenKind::Eof, line, col)),
            Some(c) => c,
        };

        let kind = match ch {
            // ── numbers ───────────────────────────────────────────
            c if c.is_ascii_digit() => self.read_number(c),

            // ── strings ───────────────────────────────────────────
            '"' => TokenKind::Str(self.read_string()?),

            // ── identifiers / keywords ────────────────────────────
            c if c.is_alphabetic() || c == '_' => self.read_ident_or_keyword(c),

            // ── two-char operators ────────────────────────────────
            '=' => if self.peek() == Some('=') { self.advance(); TokenKind::EqEq }
                   else if self.peek() == Some('>') { self.advance(); TokenKind::FatArrow }
                   else { TokenKind::Eq },
            '!' => if self.peek() == Some('=') { self.advance(); TokenKind::BangEq } else { TokenKind::Bang },
            '<' => if self.peek() == Some('=') { self.advance(); TokenKind::LtEq }
                   else if self.peek() == Some('<') { self.advance(); TokenKind::Shl }
                   else { TokenKind::Lt },
            '>' => if self.peek() == Some('=') { self.advance(); TokenKind::GtEq }
                   else if self.peek() == Some('>') { self.advance(); TokenKind::Shr }
                   else { TokenKind::Gt },
            '&' => if self.peek() == Some('&') { self.advance(); TokenKind::And } else { TokenKind::Ampersand },
            '|' => if self.peek() == Some('|') { self.advance(); TokenKind::Or } else { TokenKind::Pipe },
            '-' => if self.peek() == Some('>') { self.advance(); TokenKind::Arrow }
                   else if self.peek() == Some('=') { self.advance(); TokenKind::MinusEq }
                   else { TokenKind::Minus },
            '+' => if self.peek() == Some('=') { self.advance(); TokenKind::PlusEq } else { TokenKind::Plus },
            '*' => if self.peek() == Some('=') { self.advance(); TokenKind::StarEq } else { TokenKind::Star },
            '/' => if self.peek() == Some('=') { self.advance(); TokenKind::SlashEq } else { TokenKind::Slash },
            '.' => if self.peek() == Some('.') { self.advance(); TokenKind::DotDot } else { TokenKind::Dot },
            ':' => if self.peek() == Some(':') { self.advance(); TokenKind::ColonColon } else { TokenKind::Colon },

            // ── single-char ───────────────────────────────────────
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '%' => TokenKind::Percent,
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            '#' => TokenKind::Hash,
            '@' => TokenKind::At,

            c => return Err(LexError::UnexpectedChar(c, line, col)),
        };

        Ok(Token::new(kind, line, col))
    }
}

#[derive(Debug)]
pub enum LexError {
    UnexpectedChar(char, usize, usize),
    UnterminatedString(usize),
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnexpectedChar(c, l, col) =>
                write!(f, "[LexError] Unexpected character '{}' at {}:{}", c, l, col),
            LexError::UnterminatedString(l) =>
                write!(f, "[LexError] Unterminated string literal at line {}", l),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lex = Lexer::new("let x = 42;");
        let tokens = lex.tokenize().unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(matches!(kinds[0], TokenKind::Let));
        assert!(matches!(kinds[1], TokenKind::Ident(_)));
        assert!(matches!(kinds[2], TokenKind::Eq));
        assert!(matches!(kinds[3], TokenKind::Int(42)));
        assert!(matches!(kinds[4], TokenKind::Semicolon));
    }

    #[test]
    fn test_string_literal() {
        let mut lex = Lexer::new(r#"let s = "hello";"#);
        let tokens = lex.tokenize().unwrap();
        assert!(matches!(&tokens[3].kind, TokenKind::Str(s) if s == "hello"));
    }

    #[test]
    fn test_keywords() {
        let src = "fn if else while for struct impl enum match use pub return";
        let mut lex = Lexer::new(src);
        let tokens = lex.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::If));
        assert!(matches!(tokens[2].kind, TokenKind::Else));
    }
}
