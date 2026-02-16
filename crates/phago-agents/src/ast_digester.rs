//! AST-based code digester using tree-sitter.
//!
//! Replaces regex-based code parsing with proper AST extraction for
//! multi-language support (Rust, Python, JavaScript/TypeScript).
//!
//! Outputs the same `CodeElement` / `CodeElementKind` types as the
//! regex-based `code_digester`, so it can be used as a drop-in replacement.
//!
//! Feature-gated behind `ast` in phago-agents.

use crate::code_digester::{CodeElement, CodeElementKind};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator as _};

/// Supported languages for AST extraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodeLanguage {
    Rust,
    Python,
    JavaScript,
}

impl CodeLanguage {
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            "js" | "jsx" | "ts" | "tsx" | "mjs" => Some(Self::JavaScript),
            _ => None,
        }
    }

    /// Detect language from filename.
    pub fn from_filename(filename: &str) -> Option<Self> {
        let ext = filename.rsplit('.').next()?;
        Self::from_extension(ext)
    }

    fn tree_sitter_language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        }
    }

    fn query_source(&self) -> &'static str {
        match self {
            Self::Rust => RUST_QUERY,
            Self::Python => PYTHON_QUERY,
            Self::JavaScript => JS_QUERY,
        }
    }
}

/// Tree-sitter query for Rust symbols.
const RUST_QUERY: &str = r#"
(function_item name: (identifier) @fn_name) @function
(struct_item name: (type_identifier) @struct_name) @struct_def
(enum_item name: (type_identifier) @enum_name) @enum_def
(trait_item name: (type_identifier) @trait_name) @trait_def
(impl_item type: (type_identifier) @impl_name) @impl_def
(use_declaration argument: (_) @use_path) @use_stmt
(const_item name: (identifier) @const_name) @const_def
(mod_item name: (identifier) @mod_name) @mod_def
"#;

/// Tree-sitter query for Python symbols.
const PYTHON_QUERY: &str = r#"
(function_definition name: (identifier) @fn_name) @function
(class_definition name: (identifier) @class_name) @class_def
(import_statement) @import_stmt
(import_from_statement module_name: (dotted_name) @import_name) @import_from
"#;

/// Tree-sitter query for JavaScript/TypeScript symbols.
const JS_QUERY: &str = r#"
(function_declaration name: (identifier) @fn_name) @function
(class_declaration name: (identifier) @class_name) @class_def
(import_statement) @import_stmt
(variable_declarator name: (identifier) @const_name) @const_def
(method_definition name: (property_identifier) @method_name) @method_def
"#;

/// AST-based code element extractor.
pub struct AstDigester {
    language: CodeLanguage,
}

impl AstDigester {
    /// Create a new AST digester for the given language.
    pub fn new(language: CodeLanguage) -> Self {
        Self { language }
    }

    /// Create from a filename by detecting the language.
    pub fn from_filename(filename: &str) -> Option<Self> {
        CodeLanguage::from_filename(filename).map(|lang| Self::new(lang))
    }

    /// Extract code symbols from source using tree-sitter AST.
    pub fn extract_symbols(&self, source: &str, filename: &str) -> Vec<CodeElement> {
        let ts_lang = self.language.tree_sitter_language();

        let mut parser = Parser::new();
        parser
            .set_language(&ts_lang)
            .expect("Failed to set tree-sitter language");

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return vec![],
        };

        let query_src = self.language.query_source();
        let query = match Query::new(&ts_lang, query_src) {
            Ok(q) => q,
            Err(_) => return vec![],
        };

        let mut cursor = QueryCursor::new();
        let source_bytes = source.as_bytes();
        let root = tree.root_node();

        let mut elements = Vec::new();
        let mut matches = cursor.matches(&query, root, source_bytes);

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                let node = capture.node;
                let text = node
                    .utf8_text(source_bytes)
                    .unwrap_or("")
                    .to_string();
                let line = node.start_position().row + 1;

                let kind = match &**capture_name {
                    "fn_name" | "method_name" => Some(CodeElementKind::Function),
                    "struct_name" => Some(CodeElementKind::Struct),
                    "enum_name" => Some(CodeElementKind::Enum),
                    "trait_name" => Some(CodeElementKind::Trait),
                    "impl_name" => Some(CodeElementKind::Impl),
                    "use_path" | "import_name" => Some(CodeElementKind::Use),
                    "const_name" => Some(CodeElementKind::Const),
                    "mod_name" => Some(CodeElementKind::Module),
                    "class_name" => Some(CodeElementKind::Struct),
                    _ => None,
                };

                if let Some(kind) = kind {
                    // For use/import paths, extract just the last segment
                    let name = if kind == CodeElementKind::Use {
                        extract_last_segment(&text)
                    } else {
                        text
                    };

                    elements.push(CodeElement {
                        name,
                        kind,
                        file: filename.to_string(),
                        line,
                    });
                }
            }
        }

        // Deduplicate by (name, kind, line)
        elements.dedup_by(|a, b| a.name == b.name && a.kind == b.kind && a.line == b.line);

        elements
    }
}

/// Extract the last segment of a path like `std::collections::HashMap`.
fn extract_last_segment(path: &str) -> String {
    path.rsplit("::")
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .next()
        .unwrap_or(path)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_extracts_functions() {
        let source = r#"
pub fn hello_world() {
    println!("hello");
}

fn private_fn(x: i32) -> bool {
    x > 0
}
"#;
        let digester = AstDigester::new(CodeLanguage::Rust);
        let elements = digester.extract_symbols(source, "test.rs");
        let fns: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Function)
            .collect();
        assert_eq!(fns.len(), 2, "Should find 2 functions, got: {:?}", fns);
        assert_eq!(fns[0].name, "hello_world");
        assert_eq!(fns[1].name, "private_fn");
    }

    #[test]
    fn rust_extracts_structs_enums_traits() {
        let source = r#"
pub struct Colony {
    tick: u64,
}

enum NodeType {
    Concept,
    Document,
}

pub trait Agent {
    fn act(&self);
}
"#;
        let digester = AstDigester::new(CodeLanguage::Rust);
        let elements = digester.extract_symbols(source, "types.rs");

        let structs: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Struct)
            .collect();
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Colony");

        let enums: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Enum)
            .collect();
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "NodeType");

        let traits: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Trait)
            .collect();
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].name, "Agent");
    }

    #[test]
    fn rust_extracts_impl_blocks() {
        let source = r#"
impl Colony {
    pub fn new() -> Self { Self { tick: 0 } }
}

impl Display for Colony {
    fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }
}
"#;
        let digester = AstDigester::new(CodeLanguage::Rust);
        let elements = digester.extract_symbols(source, "impl.rs");
        let impls: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Impl)
            .collect();
        assert!(impls.len() >= 2, "Should find at least 2 impl blocks, got: {:?}", impls);
    }

    #[test]
    fn python_extracts_functions_and_classes() {
        let source = r#"
def hello():
    print("hello")

class MyModel:
    def __init__(self):
        self.x = 1

    def forward(self, input):
        return input * self.x

def helper(a, b):
    return a + b
"#;
        let digester = AstDigester::new(CodeLanguage::Python);
        let elements = digester.extract_symbols(source, "model.py");

        let fns: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Function)
            .collect();
        // hello, __init__, forward, helper
        assert!(fns.len() >= 3, "Should find at least 3 functions, got: {:?}", fns);

        let classes: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Struct)
            .collect();
        assert_eq!(classes.len(), 1, "Should find 1 class");
        assert_eq!(classes[0].name, "MyModel");
    }

    #[test]
    fn javascript_extracts_functions_and_classes() {
        let source = r#"
function greet(name) {
    return `Hello, ${name}!`;
}

class Component {
    constructor() {
        this.state = {};
    }

    render() {
        return null;
    }
}
"#;
        let digester = AstDigester::new(CodeLanguage::JavaScript);
        let elements = digester.extract_symbols(source, "app.js");

        let fns: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Function)
            .collect();
        assert!(
            fns.iter().any(|f| f.name == "greet"),
            "Should find 'greet' function, got: {:?}",
            fns
        );

        let classes: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Struct)
            .collect();
        assert_eq!(classes.len(), 1, "Should find 1 class");
        assert_eq!(classes[0].name, "Component");
    }

    #[test]
    fn language_detection_from_filename() {
        assert_eq!(
            CodeLanguage::from_filename("main.rs"),
            Some(CodeLanguage::Rust)
        );
        assert_eq!(
            CodeLanguage::from_filename("script.py"),
            Some(CodeLanguage::Python)
        );
        assert_eq!(
            CodeLanguage::from_filename("app.tsx"),
            Some(CodeLanguage::JavaScript)
        );
        assert_eq!(CodeLanguage::from_filename("data.csv"), None);
    }

    #[test]
    fn superset_of_regex_digester() {
        // The AST digester should find at least everything the regex one does
        let source = r#"
pub fn hello_world() {}
fn private_fn() {}
pub struct Foo {}
enum Bar {}
pub trait Baz {}
impl Foo {}
use std::collections::HashMap;
mod submodule;
"#;
        let ast = AstDigester::new(CodeLanguage::Rust);
        let ast_elements = ast.extract_symbols(source, "test.rs");
        let regex_elements = crate::code_digester::extract_code_elements(source, "test.rs");

        // AST should find at least as many elements as regex
        assert!(
            ast_elements.len() >= regex_elements.len(),
            "AST found {} elements, regex found {} — AST should be a superset",
            ast_elements.len(),
            regex_elements.len()
        );

        // Every regex element should have a corresponding AST element
        for re in &regex_elements {
            let found = ast_elements
                .iter()
                .any(|ae| ae.name == re.name && ae.kind == re.kind);
            assert!(
                found,
                "Regex found {:?} '{}' but AST did not",
                re.kind, re.name
            );
        }
    }
}
