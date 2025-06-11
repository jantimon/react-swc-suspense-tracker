#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::HashSet;
use swc_core::common::{BytePos, SourceMapper};
use swc_core::plugin::proxies::PluginSourceMapProxy;
use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::*,
        visit::{visit_mut_pass, VisitMut, VisitMutWith},
    },
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};

mod settings;

pub use settings::{Boundary, Config, Context, Environment};

const BOUNDARY_TRACKER_PACKAGE_NAME: &str = "react-swc-suspense-tracker/context";
const BOUNDARY_TRACKER_IMPORT_NAME: &str = "BoundaryTrackerSWC";
const BOUNDARY_ID_PROPERTY_NAME: &str = "boundaryId";
const BOUNDARY_NAME_PROPERTY_NAME: &str = "boundary";

struct TransformVisitor {
    config: Config,
    context: Context,
    /// Set of boundary configurations
    boundary_contexts: HashSet<Boundary>,
    /// Valid Boundary Idents
    valid_boundary_idents: HashSet<Ident>,
    /// Track if boundary imports have been added (plugin only adds one import)
    boundary_imports_added: bool,
    /// Track if we have any boundary elements to transform
    has_boundary_elements: bool,
    /// Optional source map for line number mapping
    source_map: Option<PluginSourceMapProxy>,
}

impl TransformVisitor {
    pub fn new(config: Config, context: Context, source_map: Option<PluginSourceMapProxy>) -> Self {
        let mut boundary_contexts = HashSet::new();

        // Always add Suspense from "react" as a default boundary
        boundary_contexts.insert(Boundary {
            component: "Suspense".to_string(),
            from: "react".to_string(),
        });

        // Add user-configured boundaries
        for boundary_config in config.boundaries.iter() {
            boundary_contexts.insert(boundary_config.clone());
        }

        Self {
            config,
            context,
            boundary_contexts,
            valid_boundary_idents: HashSet::new(),
            boundary_imports_added: false,
            has_boundary_elements: false,
            source_map,
        }
    }

    /// Generates a unique ID for a custom boundary element based on boundary name, file and line
    fn generate_boundary_id(&self, pos: BytePos) -> String {
        let filename = self.context.filename.clone();
        let cleaned = filename
            .strip_prefix("./")
            .or_else(|| filename.strip_prefix("/"))
            .unwrap_or(&filename)
            .replace('\\', "/");

        let line = self
            .source_map
            .as_ref()
            .map_or(0, |source_map| source_map.lookup_char_pos(pos).line);
        format!("{}:{}", cleaned, line)
    }

    /// Creates the BoundaryTracker import if needed
    fn create_boundary_tracker_import(&self) -> ModuleItem {
        ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident {
                    ctxt: Default::default(),
                    span: DUMMY_SP,
                    sym: BOUNDARY_TRACKER_IMPORT_NAME.into(),
                    optional: false,
                },
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(Str {
                span: DUMMY_SP,
                value: BOUNDARY_TRACKER_PACKAGE_NAME.into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: ImportPhase::Evaluation,
        }))
    }

    /// Processes boundary imports: collects boundary identifier contexts
    fn process_boundary_import(&mut self, import_decl: &mut ImportDecl) {
        let Str { value, .. } = *import_decl.src.clone();

        // Check each configured boundary (including the default Suspense) to see if this import matches
        for boundary_config in &self.boundary_contexts {
            if value == boundary_config.from {
                // This import is from a package that has boundaries
                for spec in &import_decl.specifiers {
                    if let ImportSpecifier::Named(named) = spec {
                        // Get the external/imported name
                        let external_name = named
                            .imported
                            .as_ref()
                            .map(|imported| match imported {
                                ModuleExportName::Ident(ident) => &ident.sym,
                                ModuleExportName::Str(str_lit) => &str_lit.value,
                            })
                            .unwrap_or(&named.local.sym);

                        if *external_name == boundary_config.component {
                            self.valid_boundary_idents.insert(named.local.clone());
                        }
                    }
                }
            }
        }
    }

    /// Checks if a JSX element is a boundary that should be transformed
    fn get_element_boundary_ident(&self, jsx_element: &JSXElement) -> Option<Ident> {
        if let JSXElementName::Ident(ident) = &jsx_element.opening.name {
            println!(
                "Ident {:?} and Contexts {:?}",
                ident, self.valid_boundary_idents
            );
            // Check if this is a valid boundary identifier
            if self
                .valid_boundary_idents
                .iter()
                .any(|valid_ident| *valid_ident.sym == ident.sym && valid_ident.ctxt == ident.ctxt)
            {
                return Some(ident.clone());
            }
        }
        None
    }
}

impl VisitMut for TransformVisitor {
    fn visit_mut_module_items(&mut self, module_items: &mut Vec<ModuleItem>) {
        let is_enabled = self
            .config
            .enabled
            .unwrap_or(self.context.env_name == Environment::Development);

        // Skip transformation if the plugin is disabled
        // or if the environment is not Development and the config does not explicitly enable it
        if !is_enabled {
            return;
        }

        // First pass: collect boundary imports (including Suspense from React)
        for module_item in module_items.iter_mut() {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = module_item {
                self.process_boundary_import(import_decl);
            }
        }

        // If no valid boundary identifiers were found, skip further processing
        if self.valid_boundary_idents.is_empty() {
            return;
        }

        // Replace the boundary elements with BoundaryTrackerSWC
        module_items.visit_mut_children_with(self);

        // Add required import if needed
        if self.has_boundary_elements {
            let insert_index = get_first_import_index(module_items).unwrap_or(0);

            if !self.boundary_imports_added {
                let tracker_import = self.create_boundary_tracker_import();
                module_items.insert(insert_index, tracker_import);
                self.boundary_imports_added = true;
            }
        }
    }

    fn visit_mut_jsx_element(&mut self, jsx_element: &mut JSXElement) {
        // Check if this is a boundary element (including Suspense)
        if let Some(boundary_ident) = self.get_element_boundary_ident(jsx_element) {
            self.has_boundary_elements = true;

            // Transform all boundaries to BoundaryTrackerSWC
            // Change the element name to BoundaryTrackerSWC
            jsx_element.opening.name = JSXElementName::Ident(Ident {
                ctxt: Default::default(),
                span: DUMMY_SP,
                sym: BOUNDARY_TRACKER_IMPORT_NAME.into(),
                optional: false,
            });

            // Also update closing tag if it exists
            if let Some(ref mut closing) = jsx_element.closing {
                closing.name = JSXElementName::Ident(Ident {
                    ctxt: Default::default(),
                    span: DUMMY_SP,
                    sym: BOUNDARY_TRACKER_IMPORT_NAME.into(),
                    optional: false,
                });
            }

            // Add the id prop
            let id_value = self.generate_boundary_id(jsx_element.span.lo);

            let id_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: BOUNDARY_ID_PROPERTY_NAME.into(),
                }),
                value: Some(JSXAttrValue::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: id_value.into(),
                    raw: None,
                }))),
            });

            let boundary_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: BOUNDARY_NAME_PROPERTY_NAME.into(),
                }),
                value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                    span: DUMMY_SP,
                    expr: JSXExpr::Expr(Box::new(Expr::Ident(boundary_ident))),
                })),
            });

            jsx_element.opening.attrs.push(id_attr);
            jsx_element.opening.attrs.push(boundary_attr);
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
        &mut (TransformVisitor::new(config, context, Some(metadata.source_map))),
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

    const CUSTOM_ERROR_BOUNDARY: &str = r#"import { ErrorBoundary } from "my-package-name";
function App() {
  return (
    <ErrorBoundary fallback={<ErrorFallback />}>
      <MyComponent />
    </ErrorBoundary>
  );
}"#;

    const MULTIPLE_CUSTOM_BOUNDARIES: &str = r#"import { ErrorBoundary } from "my-package-name";
import { LoadingBoundary } from "another-package";
function App() {
  return (
    <div>
      <ErrorBoundary fallback={<ErrorFallback />}>
        <Component1 />
      </ErrorBoundary>
      <LoadingBoundary fallback={<div>Loading...</div>}>
        <Component2 />
      </LoadingBoundary>
    </div>
  );
}"#;

    fn transform_visitor(environment: Environment) -> VisitMutPass<TransformVisitor> {
        visit_mut_pass(TransformVisitor::new(
            Config {
                enabled: None,
                boundaries: HashSet::new(),
            },
            Context {
                env_name: environment,
                filename: "my/file.tsx".into(),
            },
            None,
        ))
    }

    fn transform_visitor_with_boundaries(
        environment: Environment,
    ) -> VisitMutPass<TransformVisitor> {
        let mut boundaries = HashSet::new();
        boundaries.insert(Boundary {
            component: "ErrorBoundary".to_string(),
            from: "my-package-name".to_string(),
        });
        boundaries.insert(Boundary {
            component: "LoadingBoundary".to_string(),
            from: "another-package".to_string(),
        });

        visit_mut_pass(TransformVisitor::new(
            Config {
                enabled: None,
                boundaries,
            },
            Context {
                env_name: environment,
                filename: "my/file.tsx".into(),
            },
            None,
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

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_boundaries(Environment::Development),
        custom_error_boundary_transform,
        CUSTOM_ERROR_BOUNDARY
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_boundaries(Environment::Development),
        multiple_custom_boundaries_transform,
        MULTIPLE_CUSTOM_BOUNDARIES
    );
}
