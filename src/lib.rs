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

pub use settings::{Config, Context, CustomBoundarySetting, Environment}; // Added CustomBoundarySetting

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
    /// Set of SyntaxContext IDs for ErrorBoundary identifiers from custom config
    error_boundary_contexts: HashSet<SyntaxContext>,
    /// Configuration for the matched custom error boundary
    custom_boundary_config: Option<CustomBoundarySetting>,
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
            error_boundary_contexts: HashSet::new(),
            custom_boundary_config: None,
        }
    }

    /// Generates a unique ID for a boundary element based on file and line
    fn generate_boundary_id(&self, name: &str, span_lo: u32) -> String {
        let line = helpers::extract_line_number(span_lo);
        // Using the original component name in the ID for better traceability
        helpers::generate_boundary_id(&format!("{}-{}", &self.context.filename, name), line)
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
                let external_name = named
                    .imported
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
        // Skip transformation if the plugin is disabled
        // or if the environment is not Development and the config does not explicitly enable it
        if !self
            .config
            .enabled
            .unwrap_or(self.context.env_name != Environment::Development)
        {
            return;
        }

        // First pass: Process imports
        // - Collect React Suspense imports and find React import position
        // - Process custom boundary imports
        for (index, module_item) in module_items.iter_mut().enumerate() {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = module_item {
                let Str { value, .. } = *import_decl.src.clone();
                if value == "react" {
                    self.react_import_position = Some(index);
                    self.process_react_import(import_decl);
                }
                // Process for custom boundaries even if not "react" import
                self.process_custom_boundary_imports(import_decl);
            }
        }

        // Second pass: transform JSX elements
        // (Suspense transformations depend on react_suspense_contexts)
        // (Error Boundary transformations depend on error_boundary_contexts)
        if !self.react_suspense_contexts.is_empty() || !self.error_boundary_contexts.is_empty() {
            module_items.visit_mut_children_with(self);
        }

        // Third pass: add SuspenseTracker import if needed
        if self.has_suspense_elements && !self.suspense_tracker_imported {
            let tracker_import = self.create_suspense_tracker_import();

            // Insert after React import if we found one, otherwise at the beginning
            let insert_index = self
                .react_import_position
                .map(|i| i + 1)
                .or_else(|| get_first_import_index(module_items))
                .unwrap_or(0);

            module_items.insert(insert_index, tracker_import);
            self.suspense_tracker_imported = true;
        }
    }

    fn visit_mut_jsx_element(&mut self, jsx_element: &mut JSXElement) {
        let mut transformed_to_custom_boundary = false;

        // Check for Custom Error Boundary transformation first
        if let Some(custom_boundary_config) = &self.custom_boundary_config {
            if let JSXElementName::Ident(ident) = &jsx_element.opening.name {
                if self.error_boundary_contexts.contains(&ident.ctxt) {
                    // This JSX element matches a configured custom error boundary
                    transformed_to_custom_boundary = true;

                    // Change the element name to "CustomBoundary" (unhygienic)
                    let custom_boundary_ident = Ident::new(
                        "CustomBoundary".into(),
                        DUMMY_SP.with_ctxt(SyntaxContext::empty()),
                    );
                    jsx_element.opening.name = JSXElementName::Ident(custom_boundary_ident.clone());
                    if let Some(ref mut closing) = jsx_element.closing {
                        closing.name = JSXElementName::Ident(custom_boundary_ident);
                    }

                    // Add `component` attribute with the original ErrorBoundary's identifier
                    let component_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
                        span: DUMMY_SP,
                        name: JSXAttrName::Ident(Ident::new("component".into(), DUMMY_SP)),
                        value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
                            span: DUMMY_SP,
                            expr: JSXExpr::Expr(Box::new(Expr::Ident(ident.clone()))),
                        })),
                    });
                    jsx_element.opening.attrs.push(component_attr);

                    // Add `id` attribute
                    let id_value =
                        self.generate_boundary_id(&ident.sym, jsx_element.span.lo.0);
                    let id_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
                        span: DUMMY_SP,
                        name: JSXAttrName::Ident(Ident::new("id".into(), DUMMY_SP)),
                        value: Some(JSXAttrValue::Lit(Lit::Str(Str {
                            span: DUMMY_SP,
                            value: id_value.into(),
                            raw: None,
                        }))),
                    });
                    jsx_element.opening.attrs.push(id_attr);

                    // Potentially, we might need to mark that a "CustomBoundary" import is needed,
                    // similar to has_suspense_elements, if it's not globally available.
                    // For now, this is outside the scope of this specific change.
                }
            }
        }

        // If not transformed to CustomBoundary, check for React Suspense transformation
        if !transformed_to_custom_boundary && self.is_react_suspense(jsx_element) {
            self.has_suspense_elements = true;

            // Change the element name to SuspenseTracker
            jsx_element.opening.name = JSXElementName::Ident(Ident {
                ctxt: SyntaxContext::empty(), // Make it unhygienic to match global/module import
                span: DUMMY_SP,
                sym: SUSPENSE_TRACKER_IMPORT_NAME.into(),
                optional: false,
            });

            // Also update closing tag if it exists
            if let Some(ref mut closing) = jsx_element.closing {
                closing.name = JSXElementName::Ident(Ident {
                    ctxt: SyntaxContext::empty(), // Make it unhygienic
                    span: DUMMY_SP,
                    sym: SUSPENSE_TRACKER_IMPORT_NAME.into(),
                    optional: false,
                });
            }

            // Add the id prop
            let id_value = self.generate_boundary_id("Suspense", jsx_element.span.lo.0);

            let id_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
                span: DUMMY_SP,
                name: JSXAttrName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "id".into(), // Corrected: IdentName expects Ident
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

impl TransformVisitor {
    // ... (other methods like new, generate_boundary_id, create_suspense_tracker_import, process_react_import, is_react_suspense)

    /// Processes custom boundary imports: collects their identifier contexts
    fn process_custom_boundary_imports(&mut self, import_decl: &mut ImportDecl) {
        if let Some(custom_boundaries_map) = &self.config.custom_boundaries {
            let import_source = &import_decl.src.value;

            for (_key, boundary_setting) in custom_boundaries_map.iter() {
                if &boundary_setting.from == import_source {
                    // This import declaration matches a configured custom boundary source
                    for specifier in &import_decl.specifiers {
                        match specifier {
                            ImportSpecifier::Named(named_spec) => {
                                // Check if the imported component name (either original or alias)
                                // matches the configured component name.
                                let imported_name = named_spec
                                    .imported
                                    .as_ref()
                                    .map(|module_export_name| match module_export_name {
                                        ModuleExportName::Ident(ident) => &ident.sym,
                                        ModuleExportName::Str(s) => &s.value,
                                    })
                                    .unwrap_or(&named_spec.local.sym);

                                if imported_name == &boundary_setting.component {
                                    self.error_boundary_contexts
                                        .insert(named_spec.local.span.ctxt);
                                    // Store the first matching configuration.
                                    // If multiple boundaries could be matched by one import,
                                    // this picks the one iterated first in the HashMap.
                                    // A more sophisticated system might be needed for conflicts.
                                    if self.custom_boundary_config.is_none() {
                                        self.custom_boundary_config = Some(boundary_setting.clone());
                                    }
                                    // Note: We don't remove the import specifier here as the
                                    // user's code still needs the original ErrorBoundary component
                                    // to be passed as a prop to CustomBoundary.
                                }
                            }
                            ImportSpecifier::Default(default_spec) => {
                                // If the configured component name matches the default import's local name
                                if default_spec.local.sym == boundary_setting.component {
                                     self.error_boundary_contexts
                                        .insert(default_spec.local.span.ctxt);
                                    if self.custom_boundary_config.is_none() {
                                        self.custom_boundary_config = Some(boundary_setting.clone());
                                    }
                                }
                            }
                            ImportSpecifier::Namespace(_) => {
                                // Namespace imports are trickier. For now, we might ignore them
                                // or require specific naming conventions if they are to be supported.
                                // e.g., if boundary_setting.component is "MyNamespace.ErrorBoundary"
                                // This would require parsing the component string.
                                // For simplicity, let's assume named/default imports for now.
                            }
                        }
                    }
                }
            }
        }
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
    use std::collections::HashMap; // For custom_boundaries in test config
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

    fn transform_visitor_with_config(
        environment: Environment,
        custom_boundaries: Option<HashMap<String, CustomBoundarySetting>>,
    ) -> VisitMutPass<TransformVisitor> {
        visit_mut_pass(TransformVisitor::new(
            Config {
                enabled: Some(true),
                custom_boundaries,
            },
            Context {
                env_name: environment,
                filename: "my/file.tsx".into(),
            },
        ))
    }

    // Keep old helper for existing suspense tests for simplicity
    fn transform_visitor(environment: Environment) -> VisitMutPass<TransformVisitor> {
        transform_visitor_with_config(environment, None)
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

    // --- Custom Error Boundary Tests ---

    const CUSTOM_ERROR_BOUNDARY_BASIC_IMPORT: &str =
        r#"import { MyErrorBoundary } from "my-error-lib";
function App() {
  return (
    <MyErrorBoundary>
      <MyComponent />
    </MyErrorBoundary>
  );
}"#;

    const CUSTOM_ERROR_BOUNDARY_ALIASED_IMPORT: &str =
        r#"import { MyErrorBoundary as CustomEB } from "my-error-lib";
function App() {
  return (
    <CustomEB>
      <MyComponent />
    </CustomEB>
  );
}"#;

    const CUSTOM_ERROR_BOUNDARY_DEFAULT_IMPORT: &str =
        r#"import DefaultErrorBoundary from "my-error-lib";
function App() {
  return (
    <DefaultErrorBoundary>
      <MyComponent />
    </DefaultErrorBoundary>
  );
}"#;

    const CUSTOM_ERROR_BOUNDARY_MIXED_SUSPENSE: &str =
        r#"import { Suspense } from "react";
import { MyErrorBoundary } from "my-error-lib";
function App() {
  return (
    <div>
      <Suspense fallback={<Loading />}>
        <Component1 />
      </Suspense>
      <MyErrorBoundary>
        <Component2 />
      </MyErrorBoundary>
    </div>
  );
}"#;

    fn custom_boundary_config(
        key: &str,
        component_name: &str,
        from_module: &str,
    ) -> HashMap<String, CustomBoundarySetting> {
        let mut boundaries = HashMap::new();
        boundaries.insert(
            key.to_string(),
            CustomBoundarySetting {
                component: component_name.to_string(),
                from: from_module.to_string(),
            },
        );
        boundaries
    }

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(
            Environment::Development,
            Some(custom_boundary_config(
                "errorBoundary",
                "MyErrorBoundary",
                "my-error-lib"
            ))
        ),
        custom_error_boundary_transform_basic,
        CUSTOM_ERROR_BOUNDARY_BASIC_IMPORT,
        r#"import { MyErrorBoundary } from "my-error-lib";
function App() {
    return (
        <CustomBoundary component={MyErrorBoundary} id="my/file.tsx-MyErrorBoundary-L4">
            <MyComponent />
        </CustomBoundary>
    );
}"#
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(
            Environment::Development,
            Some(custom_boundary_config(
                "errorBoundary",
                "MyErrorBoundary",
                "my-error-lib"
            ))
        ),
        custom_error_boundary_transform_aliased,
        CUSTOM_ERROR_BOUNDARY_ALIASED_IMPORT,
        r#"import { MyErrorBoundary as CustomEB } from "my-error-lib";
function App() {
    return (
        <CustomBoundary component={CustomEB} id="my/file.tsx-CustomEB-L4">
            <MyComponent />
        </CustomBoundary>
    );
}"#
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(
            Environment::Development,
            Some(custom_boundary_config(
                "errorBoundary",
                "DefaultErrorBoundary",
                "my-error-lib"
            ))
        ),
        custom_error_boundary_transform_default,
        CUSTOM_ERROR_BOUNDARY_DEFAULT_IMPORT,
        r#"import DefaultErrorBoundary from "my-error-lib";
function App() {
    return (
        <CustomBoundary component={DefaultErrorBoundary} id="my/file.tsx-DefaultErrorBoundary-L4">
            <MyComponent />
        </CustomBoundary>
    );
}"#
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(Environment::Development, None),
        custom_error_boundary_no_config_no_transform,
        CUSTOM_ERROR_BOUNDARY_BASIC_IMPORT
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(
            Environment::Development,
            Some(custom_boundary_config(
                "errorBoundary",
                "OtherBoundary",
                "my-error-lib"
            ))
        ),
        custom_error_boundary_wrong_component_no_transform,
        CUSTOM_ERROR_BOUNDARY_BASIC_IMPORT
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(
            Environment::Development,
            Some(custom_boundary_config(
                "errorBoundary",
                "MyErrorBoundary",
                "other-lib"
            ))
        ),
        custom_error_boundary_wrong_module_no_transform,
        CUSTOM_ERROR_BOUNDARY_BASIC_IMPORT
    );

    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(
            Environment::Development,
            Some(custom_boundary_config(
                "errorBoundary",
                "MyErrorBoundary",
                "my-error-lib"
            ))
        ),
        custom_error_boundary_mixed_with_suspense,
        CUSTOM_ERROR_BOUNDARY_MIXED_SUSPENSE,
        r#"import { SuspenseTrackerSWC } from "react-swc-suspense-tracker/context";
import { MyErrorBoundary } from "my-error-lib";
function App() {
    return (
        <div>
            <SuspenseTrackerSWC id="my/file.tsx-Suspense-L5" fallback={<Loading />}>
                <Component1 />
            </SuspenseTrackerSWC>
            <CustomBoundary component={MyErrorBoundary} id="my/file.tsx-MyErrorBoundary-L8">
                <Component2 />
            </CustomBoundary>
        </div>
    );
}"#
    );

    // Test that if custom_boundaries is Some but empty, it doesn't break
    test!(
        module,
        tsx_syntax(),
        |_| transform_visitor_with_config(Environment::Development, Some(HashMap::new())),
        custom_error_boundary_empty_config_no_transform,
        CUSTOM_ERROR_BOUNDARY_BASIC_IMPORT
    );
}
