use helpers::{
    as_unique_identifier,
    Resolve,
    SyncRef,
};
use indexmap::IndexMap;
use language::{
    Attribute,
    AttributeAST,
    CompoundDataType,
    DataType,
    DataTypeAST,
    Field,
    FieldAST,
    find_attribute,
    FunctionDefinitionAST,
    ItemPath,
};
use lexeme_scanner::ItemPosition;
use parser_basics::Identifier;
use project_analysis::{
    Item,
    Module,
    SemanticError,
    SemanticItemType,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTypeDefinitionAST<'source> {
    pub name: Identifier<'source>,
    pub body: DataTypeAST<'source>,
}

impl<'source> Resolve<SyncRef<Module>> for DataTypeDefinitionAST<'source> {
    type Result = Item;
    type Error = SemanticError;
    fn resolve(&self, ctx: &SyncRef<Module>) -> Result<Self::Result, Vec<Self::Error>> {
        let body = self.body.resolve(ctx)?;
        let def = DataTypeDefinition {
            name: self.name.to_string(),
            body,
        };
        Ok(Item::data_type(ctx.clone(), def))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataTypeDefinition {
    pub name: String,
    pub body: DataType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDefinitionAST<'source> {
    pub name: Identifier<'source>,
    pub pos: ItemPosition,
    pub body: Vec<(Identifier<'source>, FieldAST<'source>)>,
}

impl<'source> Resolve<SyncRef<Module>> for TableDefinitionAST<'source> {
    type Result = TableDefinition;
    type Error = SemanticError;
    fn resolve(&self, ctx: &SyncRef<Module>) -> Result<Self::Result, Vec<Self::Error>> {
        let body = match as_unique_identifier(self.body.clone()) {
            Ok(map) => Arc::new(map.resolve(ctx)?),
            Err(name) => return SemanticError::duplicate_definition(
                name.item_pos(),
                name.text().to_string(),
                SemanticItemType::Field,
            )
                .into_err_vec(),
        };
        Ok(TableDefinition {
            name: self.name.to_string(),
            pos: self.pos,
            body,
            entity: None,
            primary_key: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableDefinition {
    pub name: String,
    pub pos: ItemPosition,
    pub body: Arc<IndexMap<String, Field>>,
    pub entity: Option<DataType>,
    pub primary_key: Option<DataType>,
}

impl TableDefinition {
    #[inline]
    pub fn make_entity_type(&mut self) -> DataType {
        let result;
        self.entity = match &self.entity {
            Some(data_type) => return data_type.clone(),
            None => {
                result = DataType::Compound(CompoundDataType::Structure(self.body.clone()));
                Some(result.clone())
            }
        };
        result
    }
    #[inline]
    pub fn make_primary_key_type(&mut self) -> DataType {
        let result;
        self.primary_key = match &self.primary_key {
            Some(data_type) => return data_type.clone(),
            None => {
                let mut body = IndexMap::new();
                for (name, field) in self.body.iter() {
                    let is_primary_key_part = find_attribute(
                        field.attributes.as_slice(),
                        "primary_key",
                    ).is_some();
                    if is_primary_key_part {
                        body.insert(name.clone(), field.clone());
                    }
                }
                result = DataType::Compound(CompoundDataType::Structure(Arc::new(body)));
                Some(result.clone())
            }
        };
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalItemTailAST<'source> {
    None,
    Asterisk,
    Alias(Identifier<'source>),
}

impl<'source> Into<ExternalItemTail> for ExternalItemTailAST<'source> {
    fn into(self) -> ExternalItemTail {
        match self {
            ExternalItemTailAST::None => ExternalItemTail::None,
            ExternalItemTailAST::Asterisk => ExternalItemTail::Asterisk,
            ExternalItemTailAST::Alias(identifier) => ExternalItemTail::Alias(identifier.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalItemTail {
    None,
    Asterisk,
    Alias(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalItemImportAST<'source> {
    pub path: ItemPath,
    pub tail: ExternalItemTailAST<'source>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternalItemImport {
    pub item: SyncRef<Item>,
    pub tail: ExternalItemTail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleDefinitionValueAST<'source> {
    DataType(DataTypeDefinitionAST<'source>),
    Table(TableDefinitionAST<'source>),
    Function(FunctionDefinitionAST<'source>),
    Module(ModuleDefinitionAST<'source>),
    Import(ExternalItemImportAST<'source>),
}

impl<'source> ModuleDefinitionValueAST<'source> {
    pub fn name(&'source self) -> &'source str {
        match self {
            ModuleDefinitionValueAST::DataType(def) => def.name.text(),
            ModuleDefinitionValueAST::Import(def) => {
                match &def.tail {
                    ExternalItemTailAST::None | &ExternalItemTailAST::Asterisk => {
                        def.path.path.as_path()
                            .pop_right()
                            .expect("Import's path should not be empty!")
                    }
                    ExternalItemTailAST::Alias(alias) => {
                        alias.text()
                    }
                }
            }
            ModuleDefinitionValueAST::Function(def) => def.name.text(),
            ModuleDefinitionValueAST::Table(def) => def.name.text(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleDefinitionValue {
    DataType(DataTypeDefinition),
    Table(TableDefinition),
    //    Function(FunctionDefinition),
//    Module(ModuleDefinition),
    Import(ExternalItemImport),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefinitionItemAST<'source> {
    pub public: bool,
    pub position: ItemPosition,
    pub attributes: Vec<AttributeAST<'source>>,
    pub value: ModuleDefinitionValueAST<'source>,
}

impl<'source> Resolve<SyncRef<Module>> for ModuleDefinitionItemAST<'source> {
    type Result = ();
    type Error = SemanticError;
    fn resolve(&self, ctx: &SyncRef<Module>) -> Result<Self::Result, Vec<Self::Error>> {
        let ModuleDefinitionItemAST { public, position, attributes, value } = self;
        let item = {
            let value = match value {
                ModuleDefinitionValueAST::DataType(def) => {
                    SyncRef::new(def.resolve(ctx)?)
                }
                ModuleDefinitionValueAST::Import(
                    ExternalItemImportAST { path, tail }
                ) => {
                    let mut item_path = path.path.as_path();
                    let item = match ctx.resolve_import(item_path) {
                        Some(item) => item,
                        None => return SemanticError::unresolved_item(path.pos, path.path.clone()).into_err_vec(),
                    };
                    if *tail == ExternalItemTailAST::Asterisk {
                        let item = item.read();
                        match item.get_module_ref() {
                            Some(module) => {
                                ctx.inject_import_module(module.clone());
                            }
                            None => return SemanticError::expected_item_of_another_type(
                                path.pos,
                                SemanticItemType::Module,
                                item.get_type(),
                            )
                                .into_err_vec(),
                        }
                    }
                    item
                }
                ModuleDefinitionValueAST::Function(def) => {
                    let ctx = (ctx.clone(), attributes.clone());
                    let def = def.resolve(&ctx)?;
                    SyncRef::new(Item::function(ctx.0.clone(), def))
                }
                ModuleDefinitionValueAST::Table(def) => {
                    let def = def.resolve(ctx)?;
                    SyncRef::new(Item::table(ctx.clone(), def))
                }
                ModuleDefinitionValueAST::Module(_) => {
                    return SemanticError::not_supported_yet(self.position, "file-scoped modules")
                        .into_err_vec();
                }
            };
            ModuleDefinitionItem {
                public: *public,
                position: *position,
                attributes: attributes.iter()
                    .map(|attr| attr.into())
                    .collect(),
                value,
            }
        };
        let name = value.name();
        ctx.put_item(name, item);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefinitionItem {
    pub public: bool,
    pub position: ItemPosition,
    // TODO Продумать перемещение аттрибутов дефиниции
    pub attributes: Vec<Attribute>,
    pub value: SyncRef<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefinitionAST<'source> {
    pub name: Identifier<'source>,
    pub items: Vec<ModuleDefinitionItemAST<'source>>,
}
