use man_lang::expressions::Expression;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinCondition<'source> {
    Expression(Expression<'source>),
    Using(Vec<Vec<&'source str>>),
    Natural,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinType {
    Cross,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataSource<'source> {
    Table {
        name: &'source str,
        alias: Option<&'source str>,
    },
    Join {
        join_type: JoinType,
        condition: Option<JoinCondition<'source>>,
        left: Box<DataSource<'source>>,
        right: Box<DataSource<'source>>,
    },
    // TODO SELECT query as data source
}
