use lexeme_scanner::Token;
use parser_basics::{
    comma_list,
    identifier,
    keyword,
    none,
    ParserResult,
    symbols,
    u32_literal,
};
use syntax_parser::expressions::{expression, Expression};
use syntax_parser::data_sources::data_source;
use super::*;

parser_rule!(select_distincty(i) -> bool {
    alt!(i,
        apply!(keyword, "all") => { |_| false } |
        apply!(keyword, "distinct") => { |_| true } |
        apply!(keyword, "distinctrow") => { |_| true } |
        none => { |_| false }
    )
});

parser_rule!(select_result_size(i) -> SelectionResultSize {
    alt!(i,
        apply!(keyword, "sql_small_result") => { |_| SelectionResultSize::Small } |
        apply!(keyword, "sql_big_result") => { |_| SelectionResultSize::Big } |
        apply!(keyword, "sql_buffer_result") => { |_| SelectionResultSize::Buffered } |
        none => { |_| SelectionResultSize::Usual }
    )
});

parser_rule!(select_cache(i) -> bool {
    alt!(i,
        apply!(keyword, "sql_cache") => { |_| true } |
        apply!(keyword, "sql_no_cache") => { |_| false } |
        none => { |_| false }
    )
});

parser_rule!(select_expression(i) -> SelectionExpression<'source> {
    do_parse!(i,
        expr: expression >>
        alias: opt!(do_parse!(
            apply!(keyword, "as") >>
            name: identifier >>
            (name)
        )) >>
        (SelectionExpression { expr, alias })
    )
});

parser_rule!(select_result(i) -> SelectionResult<'source> {
    alt!(i,
        apply!(symbols, "*") => { |_| SelectionResult::All }
        | apply!(comma_list, select_expression) => { |x| SelectionResult::Some(x) }
    )
});

parser_rule!(pub select_condition(i, prefix_keyword_text: &str) -> Expression<'source> {
    do_parse!(i,
        apply!(keyword, prefix_keyword_text) >>
        expr: expression >>
        (expr)
    )
});

// TODO Рассмотреть возможность использования синтаксиса tableName.* в группировке и сортировке
parser_rule!(select_sorting_order(i) -> SelectionSortingOrder {
    alt!(i,
        apply!(keyword, "asc") => { |_| SelectionSortingOrder::Asc } |
        apply!(keyword, "desc") => { |_| SelectionSortingOrder::Desc } |
        none => { |_| SelectionSortingOrder::Asc }
    )
});

parser_rule!(select_sorting_item(i) -> SelectionSortingItem<'source> {
    do_parse!(i,
        expr: expression >>
        order: select_sorting_order >>
        (SelectionSortingItem { expr, order })
    )
});

parser_rule!(pub select_sorting(i, prefix_keyword_text: &str) -> Vec<SelectionSortingItem<'source>> {
    do_parse!(i,
        apply!(keyword, prefix_keyword_text) >>
        apply!(keyword, "by") >>
        items: apply!(comma_list, select_sorting_item) >>
        (items)
    )
});

parser_rule!(select_group_by_clause(i) -> SelectionGroupByClause<'source> {
    do_parse!(i,
        sorting: apply!(select_sorting, "group") >>
        with_rollup: opt!(do_parse!(
            apply!(keyword, "with") >>
            apply!(keyword, "rollup") >>
            (())
        )) >>
        (SelectionGroupByClause { sorting, with_rollup: with_rollup.is_some() })
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
pub fn selection<'token, 'source>(input: &'token [Token<'source>]) -> ParserResult<'token, 'source, Selection<'source>> {
    do_parse!(input,
        apply!(keyword, "select") >>
        distinct: select_distincty >>
        high_priority: opt!(apply!(keyword, "high_priority")) >>
        straight_join: opt!(apply!(keyword, "straight_join")) >>
        result_size: select_result_size >>
        cache: select_cache >>
        result: select_result >>
        apply!(keyword, "from") >>
        source: data_source >>
        where_clause: opt!(apply!(select_condition, "where")) >>
        group_by_clause: opt!(select_group_by_clause) >>
        having_clause: opt!(apply!(select_condition, "having")) >>
        order_by_clause: opt!(apply!(select_sorting, "order")) >>
        limit_clause: opt!(selection_limit) >>
        (Selection {
            distinct,
            high_priority: high_priority.is_some(),
            straight_join: straight_join.is_some(),
            result_size,
            cache,
            result,
            source,
            where_clause,
            group_by_clause,
            having_clause,
            order_by_clause,
            limit_clause,
        })
    )
}