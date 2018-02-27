use helpers::extract::extract;
use lexeme_scanner::Token;
use parser_basics::{
    identifier,
    keyword,
    list,
    symbols,
    ParserResult,
};
use desc_lang::compounds::data_type;
use man_lang::expressions::expression;
use super::*;

parser_rule!(variable_definition(i) -> Statement<'source> {
    do_parse!(i,
        apply!(keyword, "let") >>
        name: identifier >>
        data_type: opt!(do_parse!(
            apply!(symbols, ":") >>
            data_type: data_type >>
            (data_type)
        )) >>
        default_value: opt!(do_parse!(
            apply!(symbols, "=") >>
            expression: expression >>
            (expression)
        )) >>
        (Statement::VariableDefinition {
            name,
            data_type,
            default_value,
        })
    )
});

parser_rule!(variable_assignment(i) -> Statement<'source> {
    do_parse!(i,
        name: identifier >>
        apply!(symbols, "=") >>
        expression: expression >>
        (Statement::VariableAssignment {
            name,
            expression,
        })
    )
});

parser_rule!(condition(i) -> Statement<'source> {
    do_parse!(i,
        apply!(keyword, "if") >>
        condition: expression >>
        then_body: map!(block, |stmt| Box::new(stmt)) >>
        else_body: opt!(map!(block, |stmt| Box::new(stmt))) >>
        (Statement::Condition {
            condition,
            then_body,
            else_body,
        })
    )
});

parser_rule!(simple_cycle(i) -> Statement<'source> {
    do_parse!(i,
        apply!(keyword, "loop") >>
        body: map!(block, |stmt| Box::new(stmt)) >>
        (Statement::Cycle {
            cycle_type: CycleType::Simple,
            body,
        })
    )
});

parser_rule!(pre_predicated_cycle(i) -> Statement<'source> {
    do_parse!(i,
        apply!(keyword, "while") >>
        predicate: expression >>
        body: map!(block, |stmt| Box::new(stmt)) >>
        (Statement::Cycle {
            cycle_type: CycleType::PrePredicated(predicate),
            body,
        })
    )
});

parser_rule!(post_predicated_cycle(i) -> Statement<'source> {
    do_parse!(i,
        apply!(keyword, "do") >>
        body: map!(block, |stmt| Box::new(stmt)) >>
        apply!(keyword, "while") >>
        predicate: expression >>
        (Statement::Cycle {
            cycle_type: CycleType::PostPredicated(predicate),
            body,
        })
    )
});

parser_rule!(cycle_control(i) -> Statement<'source> {
    do_parse!(i,
        operator: alt!(
            apply!(keyword, "break") => { |_| CycleControlOperator::Break }
            | apply!(keyword, "continue") => { |_| CycleControlOperator::Continue }
        ) >>
        name: opt!(identifier) >>
        (Statement::CycleControl {
            operator,
            name,
        })
    )
});

parser_rule!(return_stmt(i) -> Statement<'source> {
    do_parse!(i,
        apply!(keyword, "return") >>
        value: opt!(expression) >>
        (Statement::Return {
            value,
        })
    )
});

parser_rule!(block(i) -> Statement<'source> {
    do_parse!(i,
        apply!(symbols, "{") >>
        statements: apply!(list, statement, prepare!(symbols(";"))) >>
        apply!(symbols, "}") >>
        (match statements.len() {
            0 => Statement::Nothing,
            1 => {
                let mut statements = statements;
                extract(&mut statements[0])
            },
            _ => Statement::Block { statements },
        })
    )
});

parser_rule!(expr(i) -> Statement<'source> {
    do_parse!(i,
        expression: expression >>
        (Statement::Expression{
            expression,
        })
    )
});

/// Выполняет разбор императивных высказываний
pub fn statement<'token, 'source>(input: &'token [Token<'source>]) -> ParserResult<'token, 'source, Statement<'source>> {
    alt!(input,
        variable_definition
        | variable_assignment
        | condition
        | simple_cycle
        | pre_predicated_cycle
        | post_predicated_cycle
        | cycle_control
        | return_stmt
        | block
        | expr
    )
}

#[cfg(test)]
mod tests {
    use helpers::assertion::Assertion;
    use lexeme_scanner::Scanner;
    use parser_basics::parse;
    use man_lang::statements::{
        CycleType,
        statement,
        Statement,
    };

    // TODO Протестировать модуль

    #[test]
    fn simple_definition_parses_correctly() {
        let tokens = Scanner::scan("let my_first_variable: boolean = false")
            .expect("Scanner result must be ok");
        let result = parse(tokens.as_slice(), statement)
            .expect("Parser result must be ok");
        match result {
            Statement::VariableDefinition { name, ref data_type, ref default_value } => {
                assert_eq!(name, "my_first_variable");
                data_type.assert(&Some("boolean"));
                default_value.assert(&Some("false"));
            },
            o => panic!("Pattern Statement::VariableDefinition does not match this value {:?}", o),
        }
    }

    #[test]
    fn simple_not_perfect_definition_parses_correctly() {
        let tokens = Scanner::scan("let my_first_variable = false")
            .expect("Scanner result must be ok");
        let result = parse(tokens.as_slice(), statement)
            .expect("Parser result must be ok");
        match result {
            Statement::VariableDefinition { name, ref data_type, ref default_value } => {
                assert_eq!(name, "my_first_variable");
                assert_eq!(*data_type, None);
                default_value.assert(&Some("false"));
            },
            o => panic!("Pattern Statement::VariableDefinition does not match this value {:?}", o),
        }
        let tokens = Scanner::scan("let my_first_variable: boolean")
            .expect("Scanner result must be ok");
        let result = parse(tokens.as_slice(), statement)
            .expect("Parser result must be ok");
        match result {
            Statement::VariableDefinition { name, ref data_type, ref default_value } => {
                assert_eq!(name, "my_first_variable");
                data_type.assert(&Some("boolean"));
                assert_eq!(*default_value, None);
            },
            o => panic!("Pattern Statement::VariableDefinition does not match this value {:?}", o),
        }
    }

    #[test]
    fn simple_assignment_parses_correctly() {
        let tokens = Scanner::scan("super_variable = 2 + 2")
            .expect("Scanner result must be ok");
        let result = parse(tokens.as_slice(), statement)
            .expect("Parser result must be ok");
        match result {
            Statement::VariableAssignment { name, ref expression } => {
                assert_eq!(name, "super_variable");
                expression.assert("2+2");
            },
            o => panic!("Pattern Statement::VariableAssignment does not match this value {:?}", o),
        }
    }

    #[test]
    fn simple_condition_parses_correctly() {
        let tokens = Scanner::scan("if it.hasNext { it.next }")
            .expect("Scanner result must be ok");
        let result = parse(tokens.as_slice(), statement)
            .expect("Parser result must be ok");
        match result {
            Statement::Condition { ref condition, ref then_body, ref else_body } => {
                condition.assert("it.hasNext");
                match &**then_body {
                    &Statement::Expression { ref expression } => {
                        expression.assert("it.next()");
                    },
                    o => panic!("Pattern Statement::Expression does not match this value {:?}", o),
                }
                assert_eq!(*else_body, None)
            },
            o => panic!("Pattern Statement::Condition does not match this value {:?}", o),
        }
        let tokens = Scanner::scan("if it.hasNext { it.next } else { null }")
            .expect("Scanner result must be ok");
        let result = parse(tokens.as_slice(), statement)
            .expect("Parser result must be ok");
        match result {
            Statement::Condition { ref condition, ref then_body, ref else_body } => {
                condition.assert("it.hasNext");
                match &**then_body {
                    &Statement::Expression { ref expression } => {
                        expression.assert("it.next()");
                    },
                    o => panic!("Pattern Statement::Expression does not match this value {:?}", o),
                }
                match *else_body {
                    Some(ref b) => match &**b {
                        &Statement::Expression { ref expression } => {
                            expression.assert("null");
                        },
                        o => panic!("Pattern Statement::Expression does not match this value {:?}", o),
                    },
                    // FIXME Почему-то тест падает потому, что в else_body ничего нет.
                    None => panic!("Option::Some(_) != Option::None"),
                }
            },
            o => panic!("Pattern Statement::Condition does not match this value {:?}", o),
        }
    }

    #[test]
    fn simple_cycle_parses_correctly() {
        let tokens = Scanner::scan("loop { 2 + 2 }")
            .expect("Scanner result must be ok");
        let result = parse(tokens.as_slice(), statement)
            .expect("Parser result must be ok");
        match result {
            Statement::Cycle { ref cycle_type, ref body } => {
                assert_eq!(*cycle_type, CycleType::Simple);
                match &**body {
                    &Statement::Expression { ref expression } => {
                        expression.assert("2 + 2");
                    },
                    o => panic!("Pattern Statement::Expression does not match this value {:?}", o),
                }
            },
            o => panic!("Pattern Statement::Cycle does not match this value {:?}", o),
        }
    }
}
