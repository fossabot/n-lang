//! Ошибка синтаксического разбора

// TODO Применить группировку к некоторым ошибкам одинакового типа

use std::fmt::{
    Debug,
    Display,
    Result as FResult,
    Formatter,
};

use std::cmp::{
    Ord,
    Ordering,
    PartialOrd,
};

use std::mem::replace;

use lexeme_scanner::{
    TokenKindLess,
    SymbolPosition,
};

/**
    Тип, отображающий некоторый объект текста.

    Существует только для того, чтобы помочь варианту `ParserErrorKind::ExpectedGot` не размножиться
    на 8 штук только из-за необходимости вариативности отображения объектов.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserErrorTokenInfo {
    Kind(TokenKindLess),
    Object(TokenKindLess, String),
    Description(String),
}

impl Display for ParserErrorTokenInfo {
    fn fmt(&self, f: &mut Formatter) -> FResult {
        match self {
            &ParserErrorTokenInfo::Kind(ref kind) => write!(f, "{}", kind),
            &ParserErrorTokenInfo::Object(ref kind, ref msg) => write!(f, "{}({})", kind, msg),
            &ParserErrorTokenInfo::Description(ref msg) => write!(f, "{}", msg),
        }
    }
}

/**
    Тип синтаксической ошибки.
    Самая интересная часть для того, кто собрался написать ещё пару правил.
    Тип ошибки сообщает о том, что именно произошло в процессе разбора.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserErrorKind {
    /// Неожиданный конец. Сообщает о том, что лексемы закончились, но правила этого не допускают.
    UnexpectedEnd(Option<String>),
    /// Неожиданный ввод. Сообщает о том, что ожидалась лексема одного вида, а была получена - другого.
    ExpectedGot(ParserErrorTokenInfo, ParserErrorTokenInfo),
    /// Ключ не уникален. Сообщает о том, что в определении структуры находится два поля с одинаковым именем.
    KeyIsNotUnique(String),
    /// Прочая ошибка. Сообщает о том, что произошло что-то где-то за пределами парсера.
    CustomError(String),
}

impl ParserErrorKind {
    /// Конструирует новый `ParserErrorKind::UnexpectedEnd` с сообщением о том, что ожидался символ
    #[inline]
    pub fn unexpected_end_expected_debug<D: Debug>(c: D) -> Self {
        ParserErrorKind::UnexpectedEnd(Some(format!("{:?}", c)))
    }
    /// Конструирует новый `ParserErrorKind::UnexpectedEnd` с данным сообщением об ожидании
    #[inline]
    pub fn unexpected_end_expected<S: ToString>(msg: S) -> Self {
        ParserErrorKind::UnexpectedEnd(Some(msg.to_string()))
    }
    /// Конструирует новый `ParserErrorKind::UnexpectedEnd` без сообщения
    #[inline]
    pub fn unexpected_end() -> Self {
        ParserErrorKind::UnexpectedEnd(None)
    }
    /// Конструирует новый `ParserErrorKind::ExpectedGot`, содержащий инофрмацию о типе ожидаемого и полученного токенов
    #[inline]
    pub fn expected_got_kind(expected: TokenKindLess, got: TokenKindLess) -> Self {
        let a = ParserErrorTokenInfo::Kind(expected);
        let b = ParserErrorTokenInfo::Kind(got);
        ParserErrorKind::ExpectedGot(a, b)
    }
    /// Конструирует новый `ParserErrorKind::ExpectedGot`, содержащий инофрмацию о типе и тексте ожидаемого и полученного токенов
    #[inline]
    pub fn expected_got_kind_text<A: ToString, B: ToString>(expected_kind: TokenKindLess, expected_text: A, got_kind: TokenKindLess, got_text: B) -> Self {
        let a = ParserErrorTokenInfo::Object(expected_kind, expected_text.to_string());
        let b = ParserErrorTokenInfo::Object(got_kind, got_text.to_string());
        ParserErrorKind::ExpectedGot(a, b)
    }
    /// Конструирует новый `ParserErrorKind::ExpectedGot`, содержащий описание ожидаемого токена и инофрмацию о типе и тексте полученного токена
    #[inline]
    pub fn expected_got_description<A: ToString, B: ToString>(expected: A, got_kind: TokenKindLess, got_text: B) -> Self {
        let a = ParserErrorTokenInfo::Description(expected.to_string());
        let b = ParserErrorTokenInfo::Object(got_kind, got_text.to_string());
        ParserErrorKind::ExpectedGot(a, b)
    }
    /// Конструирует новый `ParserErrorKind::NomError`, содержащий сообщение об ошибке комбинатора парсеров
    #[inline]
    pub fn custom_error<A: ToString>(msg: A) -> Self {
        ParserErrorKind::CustomError(msg.to_string())
    }
    /// Конструирует новый `ParserErrorKind::KeyIsNotUnique`, содержащий сообщение имя повторяющегося ключа
    #[inline]
    pub fn key_is_not_unique<A: ToString>(msg: A) -> Self {
        ParserErrorKind::KeyIsNotUnique(msg.to_string())
    }
}

/// Типаж Display у `ParserErrorKind` служит для отображения типа ошибки в человекочитаемом виде
impl Display for ParserErrorKind {
    fn fmt(&self, f: &mut Formatter) -> FResult {
        match self {
            &ParserErrorKind::UnexpectedEnd(ref s) => {
                write!(f, "unexpected end")?;
                if let &Some(ref m) = s {
                    write!(f, ", expected: {}", m)?;
                }
                Ok(())
            },
            &ParserErrorKind::ExpectedGot(ref exp, ref got) => write!(f, "expected: {}, got: {}", exp, got),
            &ParserErrorKind::KeyIsNotUnique(ref key) => write!(f, "key {:?} is not unique", key),
            &ParserErrorKind::CustomError(ref msg) => write!(f, "{}", msg),
        }
    }
}

/// Одиночная ошибка разбора. Применяется как элемент `ParserError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserErrorItem {
    pub kind: ParserErrorKind,
    pub pos: Option<SymbolPosition>,
}

impl ParserErrorItem {
    /// Конструирует новую единицу ошибки из типа и позиции
    #[inline]
    const fn new(kind: ParserErrorKind, pos: SymbolPosition) -> Self {
        Self {
            kind,
            pos: Some(pos),
        }
    }
    /// Конструирует новую единицу ошибки из типа, но без позиции
    #[inline]
    const fn new_without_pos(kind: ParserErrorKind) -> Self {
        Self {
            kind,
            pos: None,
        }
    }
}

/// Типаж Display у `ParserErrorItem` служит для отображения ошибки в человекочитаемом виде
impl Display for ParserErrorItem {
    fn fmt(&self, f: &mut Formatter) -> FResult {
        write!(f, "{}", self.kind)?;
        if let &Some(ref pos) = &self.pos {
            write!(f, " on {}", pos)?;
        }
        Ok(())
    }
}

impl PartialOrd for ParserErrorItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.pos.partial_cmp(&other.pos)
    }
}

impl Ord for ParserErrorItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("Trying to sort error from different modules")
    }
}

/// Ошибка разбора. Может содержать несколько `ParserErrorItem`.
#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    One(ParserErrorItem),
    Many(Vec<ParserErrorItem>),
}

impl ParserError {
    /// Конструирует единичную ошибку из типа и позиции
    #[inline]
    pub const fn new(kind: ParserErrorKind, pos: SymbolPosition) -> ParserError {
        ParserError::One(
            ParserErrorItem::new(kind, pos)
        )
    }
    /// Конструирует единичную ошибку из типа, но без позиции
    #[inline]
    pub const fn new_without_pos(kind: ParserErrorKind) -> ParserError {
        ParserError::One(
            ParserErrorItem::new_without_pos(kind)
        )
    }
    /// Выполняет копирование всех хранимых ошибок в вектор и возвращает его
    #[inline]
    pub fn extract_into_vec(&self) -> Vec<ParserErrorItem> {
        match self {
            &ParserError::One(ref e) => vec![e.clone()],
            &ParserError::Many(ref v) => v.clone(),
        }
    }
    /// Выполняет поглощение другой ошибки.
    /// После выполнения текущий объект будет содержать как свои элементы, так и элементы из переданного объекта.
    pub fn append(&mut self, other_error: ParserError) {
        let result = match self {
            &mut ParserError::One(ref self_item) => {
                let self_item = self_item.clone();
                let new_vec = match other_error {
                    ParserError::One(other_item) => {
                        if self_item == other_item { return; }
                        vec![self_item, other_item]
                    },
                    ParserError::Many(mut other_vec) => {
                        if !other_vec.contains(&self_item) {
                            other_vec.push(self_item);
                        }
                        other_vec
                    },
                };
                ParserError::Many(new_vec)
            },
            &mut ParserError::Many(ref mut self_vec) => {
                match other_error {
                    ParserError::One(other_item) => {
                        if !self_vec.contains(&other_item) {
                            self_vec.push(other_item);
                        }
                    },
                    ParserError::Many(mut other_vec) => {
                        for other_item in other_vec {
                            if !self_vec.contains(&other_item) {
                                self_vec.push(other_item);
                            }
                        }
                    },
                }
                return;
            },
        };
        replace(self, result);
    }
}

/// Типаж Display у `ParserError` служит для отображения группы ошибок в человекочитаемом виде
impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter) -> FResult {
        let mut errors = self.extract_into_vec();
        errors.sort();
        writeln!(f, "There are some errors:")?;
        for (i, error) in errors.into_iter().enumerate() {
            writeln!(f, "  {}. {}", i + 1, error)?;
        }
        writeln!(f, "Solution of one of them may solve the problem.")
    }
}
