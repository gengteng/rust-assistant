use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::Arc;
use syn::spanned::Spanned;
use syn::{Attribute, ItemEnum, ItemFn, ItemImpl, ItemMacro, ItemStruct, ItemTrait};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Item {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: ItemType,
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub file: Arc<Path>,
    #[cfg_attr(feature = "utoipa", schema(value_type = RangeSchema))]
    pub line_range: RangeInclusive<NonZeroUsize>,
}

#[cfg(feature = "utoipa")]
#[derive(ToSchema)]
pub struct RangeSchema {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ItemType {
    #[default]
    All,
    Struct,
    Enum,
    Trait,
    ImplType,
    ImplTraitForType,
    Macro,
    AttributeMacro,
    Function,
    TypeAlias,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchIndexMut {
    pub structs: FnvHashMap<String, Vec<Item>>,
    pub enums: FnvHashMap<String, Vec<Item>>,
    pub traits: FnvHashMap<String, Vec<Item>>,
    pub impl_types: FnvHashMap<String, Vec<Item>>,
    pub impl_trait_for_types: FnvHashMap<String, Vec<Item>>,
    pub macros: FnvHashMap<String, Vec<Item>>,
    pub attribute_macros: FnvHashMap<String, Vec<Item>>,
    pub functions: FnvHashMap<String, Vec<Item>>,
    pub type_aliases: FnvHashMap<String, Vec<Item>>,
}

impl SearchIndexMut {
    pub fn search(&self, type_: ItemType, query: &str) -> Vec<Item> {
        let query = query.to_lowercase();
        match type_ {
            ItemType::All => {
                let mut all = Vec::new();
                all.extend(filter_items(&query, &self.structs));
                all.extend(filter_items(&query, &self.enums));
                all.extend(filter_items(&query, &self.traits));
                all.extend(filter_items(&query, &self.impl_types));
                all.extend(filter_items(&query, &self.impl_trait_for_types));
                all.extend(filter_items(&query, &self.macros));
                all.extend(filter_items(&query, &self.attribute_macros));
                all.extend(filter_items(&query, &self.functions));
                all.extend(filter_items(&query, &self.type_aliases));
                all
            }
            ItemType::Struct => filter_items(&query, &self.structs),
            ItemType::Enum => filter_items(&query, &self.enums),
            ItemType::Trait => filter_items(&query, &self.traits),
            ItemType::ImplType => filter_items(&query, &self.impl_types),
            ItemType::ImplTraitForType => filter_items(&query, &self.impl_trait_for_types),
            ItemType::Macro => filter_items(&query, &self.macros),
            ItemType::AttributeMacro => filter_items(&query, &self.attribute_macros),
            ItemType::Function => filter_items(&query, &self.functions),
            ItemType::TypeAlias => filter_items(&query, &self.type_aliases),
        }
    }
}

fn filter_items(query: &str, items: &FnvHashMap<String, Vec<Item>>) -> Vec<Item> {
    items
        .iter()
        .filter(|(name, _)| name.contains(&query))
        .map(|(_, item)| item)
        .flatten()
        .cloned()
        .collect::<Vec<Item>>()
}

pub type SearchIndex = Arc<SearchIndexMut>;

impl SearchIndexMut {
    pub fn freeze(self) -> SearchIndex {
        Arc::new(self)
    }
}

#[derive(Debug, Default)]
pub struct SearchIndexBuilder {
    index: SearchIndexMut,
}

impl SearchIndexBuilder {
    pub fn update<P: AsRef<Path>>(&mut self, file: P, content: &str) -> bool {
        let mut visitor = IndexVisitor::new(&mut self.index, file);
        if let Ok(ast) = syn::parse_file(content) {
            syn::visit::visit_file(&mut visitor, &ast);
            true
        } else {
            false
        }
    }

    pub fn finish(self) -> SearchIndex {
        self.index.freeze()
    }
}

pub struct IndexVisitor<'i> {
    index: &'i mut SearchIndexMut,
    current_file: Arc<Path>,
}

impl<'i> IndexVisitor<'i> {
    pub fn new<P: AsRef<Path>>(index: &'i mut SearchIndexMut, current_file: P) -> Self {
        IndexVisitor {
            index,
            current_file: Arc::from(current_file.as_ref()),
        }
    }

    fn create_item(
        &self,
        name: String,
        type_: ItemType,
        item_span: proc_macro2::Span,
        attrs: &[Attribute],
    ) -> Item {
        // 获取项的 span
        let mut start_line = item_span.start().line;
        let end_line = item_span.end().line;

        // 检查并调整起始行号以包含文档注释
        for attr in attrs {
            if attr.path().is_ident("doc") {
                let attr_span = attr.span();
                start_line = start_line.min(attr_span.start().line);
            }
        }

        let start_line = NonZeroUsize::new(start_line).unwrap_or(NonZeroUsize::MIN);
        let end_line = NonZeroUsize::new(end_line).unwrap_or(NonZeroUsize::MAX);

        Item {
            name,
            type_,
            file: self.current_file.clone(),
            line_range: start_line..=end_line,
        }
    }
}

impl<'i, 'ast> syn::visit::Visit<'ast> for IndexVisitor<'i> {
    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        let name = i.ident.to_string();
        let item = self.create_item(name, ItemType::Enum, i.span(), &i.attrs);
        self.index
            .enums
            .entry(item.name.to_lowercase())
            .or_default()
            .push(item);
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if is_attribute_macro(&i.attrs) {
            let name = i.sig.ident.to_string();
            let item = self.create_item(name, ItemType::AttributeMacro, i.span(), &i.attrs);
            self.index
                .attribute_macros
                .entry(item.name.to_lowercase())
                .or_default()
                .push(item);
        } else {
            let name = i.sig.ident.to_string();
            let item = self.create_item(name, ItemType::Function, i.span(), &i.attrs);
            self.index
                .functions
                .entry(item.name.to_lowercase())
                .or_default()
                .push(item);
        }
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let self_ty = &i.self_ty;

        match &i.trait_ {
            Some((_, path, _)) => {
                // impl Trait for Type
                let impl_name = format!(
                    "impl {} for {}",
                    quote::quote! { #path },
                    quote::quote! { #self_ty }
                );
                let item =
                    self.create_item(impl_name, ItemType::ImplTraitForType, i.span(), &i.attrs);
                self.index
                    .impl_trait_for_types
                    .entry(item.name.to_lowercase())
                    .or_default()
                    .push(item);
            }
            None => {
                // impl Type
                let impl_name = format!("impl {}", quote::quote! { #self_ty });
                let item = self.create_item(impl_name, ItemType::ImplType, i.span(), &i.attrs);
                self.index
                    .impl_types
                    .entry(item.name.to_lowercase())
                    .or_default()
                    .push(item);
            }
        };
    }

    fn visit_item_macro(&mut self, i: &'ast ItemMacro) {
        if let Some(ident) = &i.ident {
            let name = ident.to_string();
            let item = self.create_item(name, ItemType::Macro, i.span(), &i.attrs);
            self.index
                .macros
                .entry(item.name.to_lowercase())
                .or_default()
                .push(item);
        }
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let name = i.ident.to_string();
        let item = self.create_item(name, ItemType::Struct, i.span(), &i.attrs);
        self.index
            .structs
            .entry(item.name.to_lowercase())
            .or_default()
            .push(item);
    }

    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        let name = i.ident.to_string();
        let item = self.create_item(name, ItemType::Trait, i.span(), &i.attrs);
        self.index
            .traits
            .entry(item.name.to_lowercase())
            .or_default()
            .push(item);
    }

    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        let name = i.ident.to_string();
        let item = self.create_item(name, ItemType::TypeAlias, i.span(), &i.attrs);
        self.index
            .type_aliases
            .entry(item.name.to_lowercase())
            .or_default()
            .push(item);
    }
}

fn is_attribute_macro(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        // check proc_macro_attribute
        attr.path().is_ident("proc_macro_attribute")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_index() {
        let p = PathBuf::from("data/src.rs");
        let content = std::fs::read_to_string(p.as_path()).expect("read file");
        let mut index_builder = SearchIndexBuilder::default();
        index_builder.update(p.as_path(), &content);
        let index = index_builder.finish();
        println!("{:#?}", index);
        println!("{:?}", index.search(ItemType::All, "trait"));
    }
}
