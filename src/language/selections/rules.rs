use language::{
    data_source,
    expression,
    ExpressionAST,
};
use lexeme_scanner::Token;
use parser_basics::{
    comma_list,
    identifier,
//    Identifier,
    item_position,
    keyword,
    none,
    ParserResult,
    symbol_position,
    symbols,
    u32_literal,
};
use super::*;

parser_rule!(select_distincty(i) -> bool {
    alt!(i,
        apply!(keyword, "all") => { |_| false } |
        apply!(keyword, "distinct") => { |_| true } |
        apply!(keyword, "distinctrow") => { |_| true } |
        none => { |_| false }
    )
});

parser_rule!(select_expression(i) -> SelectionExpressionAST<'source> {
    do_parse!(i,
        expr: expression >>
        alias: opt!(do_parse!(
            apply!(keyword, "as") >>
            name: identifier >>
            (name)
        )) >>
        (SelectionExpressionAST { expr, alias })
    )
});

parser_rule!(select_result(i) -> SelectionResultAST<'source> {
    alt!(i,
        do_parse!(
            begin: symbol_position >>
            apply!(symbols, "*") >>
            pos: apply!(item_position, begin) >>
            (SelectionResultAST::All(pos))
        )
        | apply!(comma_list, select_expression) => { |x| SelectionResultAST::Some(x) }
    )
});

parser_rule!(pub select_condition(i, prefix_keyword_text: &'source str) -> ExpressionAST<'source> {
    do_parse!(i,
        begin: symbol_position >>
        apply!(keyword, prefix_keyword_text) >>
        expr: expression >>
        ({
            let mut expr = expr;
            expr.pos.begin = begin;
            expr
        })
    )
});

parser_rule!(select_sorting_order(i) -> SelectionSortingOrder {
    alt!(i,
        apply!(keyword, "asc") => { |_| SelectionSortingOrder::Asc } |
        apply!(keyword, "desc") => { |_| SelectionSortingOrder::Desc } |
        none => { |_| SelectionSortingOrder::Asc }
    )
});

parser_rule!(select_sorting_item(i) -> SelectionSortingItemAST<'source> {
    do_parse!(i,
        expr: expression >>
        order: select_sorting_order >>
        (SelectionSortingItemAST { expr, order })
    )
});

parser_rule!(pub select_sorting(i, prefix_keyword_text: &'source str) -> Vec<SelectionSortingItemAST<'source>> {
    do_parse!(i,
        apply!(keyword, prefix_keyword_text) >>
        apply!(keyword, "by") >>
        items: apply!(comma_list, select_sorting_item) >>
        (items)
    )
});

parser_rule!(select_group_by_clause(i) -> SelectionGroupByClauseAST<'source> {
    do_parse!(i,
        begin: symbol_position >>
        sorting: apply!(select_sorting, "group") >>
        with_rollup: opt!(do_parse!(
            apply!(keyword, "with") >>
            apply!(keyword, "rollup") >>
            (())
        )) >>
        pos: apply!(item_position, begin) >>
        (SelectionGroupByClauseAST { sorting, with_rollup: with_rollup.is_some(), pos })
    )
});

parser_rule!(selection_limit(i) -> SelectionLimit {
    do_parse!(i,
        apply!(keyword, "limit") >>
        a: u32_literal >>
        x: alt!(
            do_parse!(
                apply!(symbols, ",") >>
                count: u32_literal >>
                (SelectionLimit { offset: Some(a), count })
            )
            | do_parse!(
                apply!(keyword, "offset") >>
                offset: u32_literal >>
                (SelectionLimit { offset: Some(offset), count: a })
            )
            | none => { |_| SelectionLimit { offset: None, count: a } }
        ) >>
        (x)
    )
});

/// Функция, выполняющая разбор запроса выборки
pub fn selection<'token, 'source>(input: &'token [Token<'source>]) -> ParserResult<'token, 'source, SelectionAST<'source>> {
    do_parse!(input,
        begin: symbol_position >>
        apply!(keyword, "select") >>
        distinct: select_distincty >>
        result: select_result >>
        apply!(keyword, "from") >>
        source: data_source >>
        where_clause: opt!(apply!(select_condition, "where")) >>
        group_by_clause: opt!(select_group_by_clause) >>
        having_clause: opt!(apply!(select_condition, "having")) >>
        order_by_clause: opt!(apply!(select_sorting, "order")) >>
        limit_clause: opt!(selection_limit) >>
        pos: apply!(item_position, begin) >>
        (SelectionAST {
            distinct,
            result,
            source,
            where_clause,
            group_by_clause,
            having_clause,
            order_by_clause,
            limit_clause,
            pos,
        })
    )
}
