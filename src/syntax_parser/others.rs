use lexeme_scanner::Token;
use parser_basics::{
    identifier,
    ParserResult,
    symbols,
};

/// Реализует разбор "пути" элементов, разделённых делителем. Отличается от списка тем, что не позволяет "замыкающий делитель".
pub fn path<'token, 'source>(input: &'token [Token<'source>], delimiter: &str) -> ParserResult<'token, 'source, Vec<&'source str>> {
    do_parse!(input,
        first: identifier >>
        others: many0!(do_parse!(
            apply!(symbols, delimiter) >>
            element: identifier >>
            (element)
        )) >>
        ({
            let mut result = others;
            result.insert(0, first);
            #[cfg(feature="parser_trace")]
            trace!("Path found: {:?}", result);
            result
        })
    )
}

/// Реализует разбор "пути свойства" (напр., "foo.bar.baz")
pub fn property_path<'token, 'source>(input: &'token [Token<'source>]) -> ParserResult<'token, 'source, Vec<&'source str>> {
    path(input, ".")
}

/// Реализует разбор "пути модуля" (напр., "foo::bar::baz")
pub fn module_path<'token, 'source>(input: &'token [Token<'source>]) -> ParserResult<'token, 'source, Vec<&'source str>> {
    path(input, "::")
}