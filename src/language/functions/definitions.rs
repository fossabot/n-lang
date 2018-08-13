use helpers::{
    BlockFormatter,
    CodeFormatter,
    Extractor,
    Generate,
    NameUniquer,
    Path,
    PathBuf,
    Resolve,
    SyncRef,
    TSQL,
    TSQLParameters,
};
use indexmap::IndexMap;
use language::{
    AttributeAST,
    DataType,
    DataTypeAST,
    Expression,
    FieldPrimitive,
    find_attribute_ast,
    Statement,
    StatementAST,
    TableDefinition,
};
use lexeme_scanner::ItemPosition;
use parser_basics::Identifier;
use project_analysis::{
    FunctionContext,
    FunctionVariable,
    FunctionVariableScope,
    Item,
    Module,
    SemanticError,
    SemanticItemType,
    StatementFlowControlJumping,
    StatementFlowControlPosition,
};
use std::fmt::{
    self,
    Write,
};

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBodyAST<'source> {
    External,
    Implementation(StatementAST<'source>),
}

impl<'source> Resolve<SyncRef<FunctionVariableScope>> for FunctionBodyAST<'source> {
    type Result = FunctionBody;
    type Error = SemanticError;
    fn resolve(&self, ctx: &SyncRef<FunctionVariableScope>) -> Result<Self::Result, Vec<Self::Error>> {
        let result = match self {
            FunctionBodyAST::External => FunctionBody::External,
            FunctionBodyAST::Implementation(stmt) => FunctionBody::Implementation(stmt.resolve(ctx)?),
        };
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBody {
    External,
    Implementation(Statement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinitionAST<'source> {
    pub name: Identifier<'source>,
    pub arguments: Vec<(Identifier<'source>, DataTypeAST<'source>)>,
    pub result: Option<DataTypeAST<'source>>,
    pub body: FunctionBodyAST<'source>,
    pub pos: ItemPosition,
}

impl<'source> Resolve<(SyncRef<Module>, Vec<AttributeAST<'source>>)> for FunctionDefinitionAST<'source> {
    type Result = FunctionDefinition;
    type Error = SemanticError;
    fn resolve(&self, ctx: &(SyncRef<Module>, Vec<AttributeAST<'source>>)) -> Result<Self::Result, Vec<Self::Error>> {
        let context = FunctionContext::new(ctx.0.clone());
        let root = context.root();
        let mut errors = Vec::new();

        let mut arguments = IndexMap::new();
        for (identifier, data_type) in self.arguments.iter() {
            let name = identifier.text();
            let position = identifier.item_pos();
            if arguments.contains_key(name) {
                errors.push(SemanticError::duplicate_definition(
                    position,
                    name.to_string(),
                    SemanticItemType::Variable,
                ));
                continue;
            }
            let data_type = match data_type.resolve(&ctx.0) {
                Ok(data_type) => data_type,
                Err(mut sub_errors) => {
                    errors.append(&mut sub_errors);
                    continue;
                }
            };
            let var = match root.new_variable(position, name.to_string(), Some(data_type)) {
                Ok(var) => var,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            var.make_read_only();
            var.mark_as_argument();
            arguments.insert(name.to_string(), var);
        }

        let (result_pos, result) = match &self.result {
            Some(data_type) => (data_type.pos, data_type.resolve(&ctx.0)?),
            None => (self.pos, DataType::Void),
        };

        let body = self.body.resolve(&root)?;

        if let FunctionBody::Implementation(body) = &body {
            let body_jumping = body.jumping_check(StatementFlowControlPosition::new(), &result)?;
            if (body_jumping != StatementFlowControlJumping::AlwaysReturns)
                && (result != DataType::Void) {
                return SemanticError::not_all_branches_returns(body.pos)
                    .into_err_vec();
            }
        }

        let is_lite_weight = match &body {
            FunctionBody::External => {
                find_attribute_ast(&ctx.1, "is_lite_weight").is_some()
            }
            FunctionBody::Implementation(stmt) => {
                stmt.is_lite_weight()
            }
        };

        if !is_lite_weight && result.as_array().is_some() {
            return SemanticError::not_allowed_inside(result_pos, "array type", "function with side effects")
                .into_err_vec();
        }

        Ok(FunctionDefinition {
            name: self.name.to_string(),
            arguments,
            result,
            body,
            context,
            is_lite_weight,
            pos: self.pos,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
    pub arguments: IndexMap<String, SyncRef<FunctionVariable>>,
    pub result: DataType,
    pub body: FunctionBody,
    pub context: SyncRef<FunctionContext>,
    pub is_lite_weight: bool,
    pub pos: ItemPosition,
}

impl FunctionDefinition {
    pub fn fmt_primitives_as_args(
        mut f: BlockFormatter<impl fmt::Write>,
        primitives: Vec<FieldPrimitive>,
        context: &mut TSQLFunctionContext,
        is_automatic: bool,
        last_comma: bool,
        is_output: bool,
    ) -> fmt::Result {
        let mut arguments = primitives.into_iter().peekable();
        while let Some(primitive) = arguments.next() {
            let mut line = f.line()?;
            if !is_automatic {
                write!(line, "@{}", primitive.path)?;
            } else {
                write!(line, "[{}]", primitive.path)?;
            }
            write!(line, " {}", TSQL(&primitive.field_type, context.parameters.clone()))?;
            if is_output {
                line.write(" OUTPUT")?;
            }
            if last_comma || arguments.peek().is_some() {
                line.write(", ")?;
            }
        }
        Ok(())
    }
    pub fn fmt_arguments(
        mut f: BlockFormatter<impl fmt::Write>,
        context: &mut TSQLFunctionContext,
    ) -> fmt::Result {
        let is_procedure = !context.function.is_lite_weight;
        if !is_procedure {
            f.write_line('(')?;
        }
        let mut sub_f = f.sub_block();
        {
            let mut arguments = context.function.arguments.iter().peekable();
            while let Some((argument_name, argument)) = arguments.next() {
                let mut argument_guard = argument.write();
                let mut prefix = PathBuf::new("#");
                let new_name = context.names.add_name(argument_guard.name().into());
                argument_guard.set_name(new_name);
                prefix.push(argument_name.as_str());
                let primitives = argument_guard.data_type()
                    .expect("Function argument cannot have unknown data-type")
                    .primitives(prefix);
                FunctionDefinition::fmt_primitives_as_args(
                    sub_f.clone(),
                    primitives,
                    context,
                    false,
                    is_procedure || arguments.peek().is_some(),
                    false,
                )?;
            }
        }
        if is_procedure {
            let table = if context.function.result.can_be_table() {
                context.function.result.as_table_type(context.make_result_variable_prefix())
            } else {
                None
            };
            if let Some(primitives) = table {
                FunctionDefinition::fmt_primitives_as_args(
                    sub_f,
                    primitives,
                    context,
                    false,
                    false,
                    true,
                )?;
            } else {
                let mut line = sub_f.line()?;
                line.write(format_args!("@{} ", context.make_result_variable_name()))?;
                if let Some(result) = context.function.result.as_primitive() {
                    line.write(TSQL(&result, context.parameters.clone()))?;
                } else {
                    line.write("bit")?;
                }
                line.write(" OUTPUT")?;
            }
        } else {
            let table = if context.function.result.can_be_table() {
                context.function.result.as_table_type(PathBuf::new("#"))
            } else {
                None
            };
            if let Some(primitives) = table {
                f.write_line(format_args!(") RETURNS @{} TABLE (", context.make_result_variable_name()))?;
                FunctionDefinition::fmt_primitives_as_args(
                    sub_f,
                    primitives,
                    context,
                    true,
                    false,
                    false,
                )?;
                f.write_line(')')?;
            } else {
                if let Some(result) = context.function.result.as_primitive() {
                    f.write_line(format_args!(") RETURNS {}", TSQL(&result, context.parameters.clone())))?;
                } else {
                    f.write_line(") RETURNS bit")?;
                }
            }
        }
        Ok(())
    }
    pub fn fmt_head(
        mut f: BlockFormatter<impl fmt::Write>,
        context: &mut TSQLFunctionContext,
    ) -> fmt::Result {
        let sub_f = f.sub_block();
        // TODO Добавить переменную-результат в контекст (в случае табличных данных на выходе)
        let class = if context.function.is_lite_weight { "FUNCTION" } else { "PROCEDURE" };
        f.write_line(format_args!("CREATE OR ALTER {} dbo.[{}]", class, context.make_function_name().data))?;
        FunctionDefinition::fmt_arguments(sub_f.clone(), context)
    }
    pub fn fmt_variable(
        mut f: BlockFormatter<impl fmt::Write>,
        context: &mut TSQLFunctionContext,
        var: &FunctionVariable,
    ) -> fmt::Result {
        if var.is_automatic() || var.is_argument() { return Ok(()); }
        // TODO Адекватный проброс ошибок наверх
        let data_type = var.data_type()
            .expect("Variable must have determined data-type in generate-time");
        if let DataType::Array(sub_type) = data_type {
            f.write_line(format_args!("DECLARE @{} TABLE (", var.name()))?;
            TableDefinition::fmt_primitives_as_columns(
                f.sub_block(),
                context.parameters.clone(),
                sub_type.primitives(PathBuf::new("#")),
                false,
                None,
            )?;
            f.write_line(");")?;
        } else {
            let mut prefix = PathBuf::new("#");
            prefix.push(var.name());
            for primitive in data_type.primitives(prefix) {
                f.write_line(format_args!("DECLARE @{} {};", primitive.path, TSQL(&primitive.field_type, context.parameters.clone())))?;
            }
        }
        Ok(())
    }
    pub fn fmt_body(
        mut f: BlockFormatter<impl fmt::Write>,
        context: &mut TSQLFunctionContext,
    ) -> fmt::Result {
        let body = match &context.function.body {
            FunctionBody::Implementation(stmt) => stmt,
            FunctionBody::External => return Ok(()),
        };

        f.write_line("AS BEGIN")?;
        let mut sub_f = f.sub_block();

        for variable in context.function.context.get_all_variables() {
            if variable.is_automatic() || variable.is_argument() { continue; }
            let mut variable_guard = variable.write();
            let new_name = context.names.add_name(variable_guard.name().into());
            variable_guard.set_name(new_name);
            FunctionDefinition::fmt_variable(sub_f.clone(), context, &*variable_guard)?;
        }

        {
            let array;
            let statements = if let Some(statements) = body.as_block() {
                statements
            } else {
                array = [body.clone()];
                &array[..]
            };
            Statement::fmt_block_without_parens(
                sub_f.clone(),
                context,
                statements,
            )?;
        }

        if context.function.result == DataType::Void {
            if let Some(var_name) = &context.result_variable_name {
                sub_f.write_line(format_args!("SET @{} = 0;", var_name))?;
            }
        }
        if context.function.is_lite_weight {
            if context.function.result.as_primitive().is_some() {
                sub_f.write_line("RETURN 0;")?;
            } else {
                sub_f.write_line("RETURN;")?;
            }
        }

        f.write_line("END")
    }
}

impl<'a> Generate<TSQLParameters<'a>> for FunctionDefinition {
    fn fmt(&self, f: BlockFormatter<impl fmt::Write>, parameters: TSQLParameters<'a>) -> fmt::Result {
        let mut context = TSQLFunctionContext::new(self, parameters);
        FunctionDefinition::fmt_head(f.clone(), &mut context)?;
        FunctionDefinition::fmt_body(f, &mut context)
    }
}

#[derive(Debug, Clone)]
pub struct TSQLFunctionContext<'a, 'b> {
    pub function: &'a FunctionDefinition,
    pub parameters: TSQLParameters<'b>,
    pub names: NameUniquer,
    pub function_name: Option<PathBuf>,
    pub result_variable_name: Option<String>,
    // TODO Учесть пре-вызовы перед каждой вставкой выражения
    pub temp_vars_scope: SyncRef<FunctionVariableScope>,
    pub pre_calc_calls: Vec<String>,
}

impl<'a, 'b> TSQLFunctionContext<'a, 'b> {
    pub fn new(function: &'a FunctionDefinition, parameters: TSQLParameters<'b>) -> Self {
        let temp_vars_scope = function.context.root().child();
        Self {
            function,
            parameters,
            names: NameUniquer::new(),
            function_name: None,
            result_variable_name: None,
            temp_vars_scope,
            pre_calc_calls: Vec::new(),
        }
    }
    pub fn make_function_name(&mut self) -> Path {
        if self.function_name.is_none() {
            let mut path = self.parameters.module_path.into_buf();
            path.push(self.function.name.as_str());
            self.function_name = Some(path)
        }
        match &self.function_name {
            Some(function_name) => return function_name.as_path(),
            None => unreachable!()
        }
    }
    pub fn make_result_variable_name(&mut self) -> &str {
        if self.result_variable_name.is_none() {
            self.result_variable_name = Some(self.names.add_name("return_value".into()));
        }
        match &self.result_variable_name {
            Some(result_variable_name) => result_variable_name.as_str(),
            None => unreachable!(),
        }
    }
    pub fn make_result_variable_prefix(&mut self) -> PathBuf {
        let mut prefix = PathBuf::new("#");
        prefix.push(self.make_result_variable_name());
        prefix
    }
    pub fn add_pre_calc_call(&mut self, function: &SyncRef<Item>, arguments: &[Expression]) -> Result<SyncRef<FunctionVariable>, fmt::Error> {
        let result_name = self.names.add_name("t".into());
        let result_data_type = {
            let function_guard = function.read();
            let inner_function = function_guard.get_function()
                .expect("Not-functions in function calls should not exist at generate-time");
            inner_function.result.clone()
        };
        let var = self.temp_vars_scope.new_variable(
            self.function.pos,
            result_name,
            Some(result_data_type),
        )
            .expect("Temp variable should not fail while initializing");
        var.make_read_only();
        let mut buffer = String::new();
        {
            let mut code_formatter = CodeFormatter::new(&mut buffer);
            code_formatter.indent_size = 4;
            let f = code_formatter.root_block();
            FunctionDefinition::fmt_variable(
                f.clone(),
                self,
                &*var.read(),
            )?;
            Statement::fmt_pre_call(
                f,
                Some(&var),
                function,
                arguments,
                self,
            )?;
        }
        self.pre_calc_calls.push(buffer);
        Ok(var)
    }
    #[inline]
    pub fn extract_pre_calc_calls(&mut self) -> Extractor<String> {
        Extractor::new(&mut self.pre_calc_calls)
    }
}
