#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::HashSet;
use swc_core::{
    common::{SyntaxContext, DUMMY_SP},
    ecma::{
        ast::*,
        visit::{visit_mut_pass, VisitMut, VisitMutWith},
    },
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};

mod helpers;
mod settings;

pub use settings::{Config, Context, Environment};

const SUSPENSE_TRACKER_PACKAGE: &str = "react-swc-suspense-tracker/context";
const SUSPENSE_TRACKER_IMPORT_NAME: &str = "SuspenseTrackerSWC";

struct TransformVisitor {
    config: Config,
    context: Context,
    has_suspense_elements: bool,
    suspense_tracker_imported: bool,
    /// Set of SyntaxContext IDs for Suspense identifiers imported from React
    react_suspense_contexts: HashSet<SyntaxContext>,
    /// Position to insert the SuspenseTracker import (after React import)
    react_import_position: Option<usize>,
}

impl TransformVisitor {
    pub fn new(config: Config, context: Context) -> Self {
        Self {
            config,
            context,
            has_suspense_elements: false,
            suspense_tracker_imported: false,
            react_suspense_contexts: HashSet::new(),
            react_import_position: None,
        }
    }

    /// Generates a unique ID for a Suspense element based on file and line
    fn generate_suspense_id(&self, span_lo: u32) -> String {
        let line = helpers::extract_line_number(span_lo);
        helpers::generate_boundary_id(&self.context.filename, line)
    }

    /// Creates the SuspenseTracker import if needed
    fn create_suspense_tracker_import(&self) -> ModuleItem {
        ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident {
                    ctxt: Default::default(),
                    span: DUMMY_SP,
                    sym: SUSPENSE_TRACKER_IMPORT_NAME.into(),
                    optional: false,
                },
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(Str {
                span: DUMMY_SP,
                value: SUSPENSE_TRACKER_PACKAGE.into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: ImportPhase::Evaluation,
        }))
    }

    /// Processes React imports: collects Suspense identifier contexts and removes them
    fn process_react_import(&mut self, import_decl: &mut ImportDecl) -> bool {
        let Str { value, .. } = *import_decl.src.clone();
        if value != "react" {
            return false;
        }

        let mut found_suspense = false;
        
        // Collect Suspense contexts and remove them from the import
        import_decl.specifiers.retain(|spec| {
            if let ImportSpecifier::Named(named) = spec {
                // Check the external/imported name, fall back to local name if not aliased
                let external_name = named.imported
                    .as_ref()
                    .map(|imported| match imported {
                        ModuleExportName::Ident(ident) => &ident.sym,
                        ModuleExportName::Str(str_lit) => &str_lit.value,
                    })
                    .unwrap_or(&named.local.sym);
                
                if external_name == "Suspense" {
                    // Store the context of this Suspense identifier
                    self.react_suspense_contexts.insert(named.local.ctxt);
                    found_suspense = true;
                    return false; // Remove from import
                }
            }
            true // Keep other imports
        });

        found_suspense
    }

    /// Checks if a JSX element uses a React Suspense identifier
    fn is_react_suspense(&self, jsx_element: &JSXElement) -> bool {
        if let JSXElementName::Ident(ident) = &jsx_element.opening.name {
            if ident.sym == "Suspense" {
                // Check if this identifier's context matches one we imported from React
                return self.react_suspense_contexts.contains(&ident.ctxt);
            }
        }
        false
    }
}

impl VisitMut for TransformVisitor {
    fn visit_mut_module_items(&mut self, module_items: &mut Vec<ModuleItem>) {
        // Skip transformation in test and production environments if disabled
        if !self.config.enabled || self.context.env_name != Environment::Development {
            return;
        }

        // First pass: collect React Suspense imports and find React import position
        for (index, module_item) in module_items.iter_mut().enumerate() {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = module_item {
                let Str { value, .. } = *import_decl.src.clone();
                if value == "react" {
                    self.react_import_position = Some(index);
                    self.process_react_import(import_decl);
                }
            }
        }

        // Second pass: transform JSX elements (only if we found React Suspense imports)
        if !self.react_suspense_contexts.is_empty() {
            module_items.visit_mut_children_with(self);
        }

        // Third pass: add SuspenseTracker import if needed
        if self.has_suspense_elements && !self.suspense_tracker_imported {
            let tracker_import = self.create_suspense_tracker_import();
            
            // Insert after React import if we found one, otherwise at the beginning
            let insert_index = self.react_import_position
                .map(|i| i + 1)
                .or_else(|| get_first_import_index(module_items))
                .unwrap_or(0);
                
            module_items.insert(insert_index, tracker_import);
            self.suspense_tracker_imported = true;
        }
    }

    fn visit_mut_jsx_element(&mut self, jsx_element: &mut JSXElement) {
        // Only transform if this is a React Suspense element
        if self.is_react_suspense(jsx_element) {
            self.has_suspense_elements = true;
            
            // Change the element name to SuspenseTracker
            jsx_element.opening.name = JSXElementName::Ident(Ident {
                ctxt: Default::default(),
                span: DUMMY_SP,
                sym: SUSPENSE_TRACKER_IMPORT_NAME.into(),
                optional: false,
            });

            // Also update closing tag if it exists
            if let Some(ref mut closing) = jsx_element.closing {
                closing.name = JSXElementName::Ident(Ident {
                    ctxt: Default::default(),
                    span: DUMMY_SP,
                    sym: SUSPENSE_TRACKER_IMPORT_NAME.into(),
                    optional: false,
                });
            }

            // Add the id prop
            let id_value = self.generate_suspense_id(jsx_element.span.lo.0);
            
            let id_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "id".into(),
                }),
                value: Some(JSXAttrValue::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: id_value.into(),
                    raw: None,
                }))),
            });

            jsx_element.opening.attrs.push(id_attr);
        }

        jsx_element.visit_mut_children_with(self);
    }
}

/// Returns the index of the first import within the module items if one exists.
fn get_first_import_index(module_items: &[ModuleItem]) -> Option<usize> {
    module_items
        .iter()
        .position(|module_item| is_import_decl(module_item).unwrap_or(false))
}

/// Checks whether a module item is an import declaration.
fn is_import_decl(module_item: &ModuleItem) -> Option<bool> {
    module_item.as_module_decl()?.as_import().map(|_| true)
}

/// Transforms a [`Program`].
///
/// # Arguments
///
/// - `program` - The SWC [`Program`] to transform.
/// - `config` - [`Config`] as JSON.
#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let config: Config = serde_json::from_str(
        &metadata
            .get_transform_plugin_config()
            .expect("failed to get plugin config for swc-plugin-suspense-tracker"),
    )
    .expect("failed to parse plugin config");

    let context = Context {
        filename: metadata
            .get_context(&TransformPluginMetadataContextKind::Filename)
            .expect("failed to get filename"),
        env_name: Environment::try_from(
            metadata
                .get_context(&TransformPluginMetadataContextKind::Env)
                .expect("failed to get env")
                .as_str(),
        )
        .expect("failed to parse environment"),
    };

    program.apply(visit_mut_pass(
        &mut (TransformVisitor::new(config, context)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_core::ecma::{
        parser::{Syntax, TsSyntax},
        transforms::testing::test,
        visit::{visit_mut_pass, VisitMutPass},
    };

    const BASIC_SUSPENSE: &str = r#"import { useEffect, Suspense } from "react";
function App() {
  return (
    <Suspense fallback={<Loading />}>
      <MyComponent />
    </Suspense>
  );
}"#;

    const MULTIPLE_SUSPENSE: &str = r#"import { Suspense } from "react";
function App() {
  return (
    <div>
      <Suspense fallback={<Loading />}>
        <Component1 />
      </Suspense>
      <Suspense fallback={<div>Loading...</div>}>
        <Component2 />
      </Suspense>
    </div>
  );
}"#;

    const NO_SUSPENSE: &str = r#"import { useEffect } from "react";
function App() {
  return <div>Hello World</div>;
}"#;

    const USER_DEFINED_SUSPENSE: &str = r#"import { useEffect } from "react";
// User's own Suspense component - should NOT be transformed
function Suspense(props) {
  return <div className="my-suspense">{props.children}</div>;
}
function App() {
  return (
    <Suspense fallback={<Loading />}>
      <MyComponent />
    </Suspense>
  );
}"#;

    const ALIASED_SUSPENSE: &str = r#"import { Suspense as MySuspense } from "react";
function App() {
  return (
    <MySuspense fallback={<Loading />}>
      <MyComponent />
    </MySuspense>
  );
}"#;

    const MIXED_SUSPENSE: &str = r#"import { Suspense as ReactSuspense } from "react";
// User's own Suspense component
function Suspense(props) {
  return <div className="my-suspense">{props.children}</div>;
}
function App() {
  return (
    <div>
      <ReactSuspense fallback={<Loading />}>
        <Component1 />
      </ReactSuspense>
      <Suspense fallback={<div>Should not transform</div>}>
        <Component2 />
      </Suspense>
    </div>
  );
}"#;

    fn transform_visitor(environment: Environment) -> VisitMutPass<TransformVisitor> {
        visit_mut_pass(TransformVisitor::new(
            Config {
                enabled: true,
            },
            Context {
                env_name: environment,
                filename: "my/file.tsx".into(),
            },
        ))
    }

    fn tsx_syntax() -> Syntax {
        Syntax::Typescript(TsSyntax {
            tsx: true,
            decorators: false,
            dts: false,
            no_early_errors: false,
            disallow_ambiguous_jsx_like: true,
        })
    }

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Development),
        basic_suspense_transform,
        BASIC_SUSPENSE
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Development),
        multiple_suspense_transform,
        MULTIPLE_SUSPENSE
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Development),
        no_suspense_no_change,
        NO_SUSPENSE
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Development),
        aliased_suspense_transform,
        ALIASED_SUSPENSE
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Development),
        user_defined_suspense_no_transform,
        USER_DEFINED_SUSPENSE
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Development),
        mixed_suspense_selective_transform,
        MIXED_SUSPENSE
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Production),
        production_no_transform,
        BASIC_SUSPENSE
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor(Environment::Test),
        test_no_transform,
        BASIC_SUSPENSE
    );
}