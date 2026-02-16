//! Code-aware digester agent for source code analysis.
//!
//! Extracts function names, type definitions, imports, and structural
//! patterns from Rust source code. Builds a code knowledge graph
//! where concepts are identifiers and edges are co-occurrence relations.

/// A code element extracted from source files.
#[derive(Debug, Clone)]
pub struct CodeElement {
    pub name: String,
    pub kind: CodeElementKind,
    pub file: String,
    pub line: usize,
}

/// Types of code elements we extract.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeElementKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Use,
    Const,
    Module,
}

impl CodeElementKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "fn",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Use => "use",
            Self::Const => "const",
            Self::Module => "mod",
        }
    }
}

/// Extract code elements from Rust source code.
pub fn extract_code_elements(source: &str, filename: &str) -> Vec<CodeElement> {
    let mut elements = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        // Function definitions
        if trimmed.starts_with("pub fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("pub(crate) fn ")
        {
            if let Some(name) = extract_identifier(trimmed, "fn ") {
                elements.push(CodeElement {
                    name,
                    kind: CodeElementKind::Function,
                    file: filename.to_string(),
                    line: line_num + 1,
                });
            }
        }

        // Struct definitions
        if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
            if let Some(name) = extract_identifier(trimmed, "struct ") {
                elements.push(CodeElement {
                    name,
                    kind: CodeElementKind::Struct,
                    file: filename.to_string(),
                    line: line_num + 1,
                });
            }
        }

        // Enum definitions
        if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
            if let Some(name) = extract_identifier(trimmed, "enum ") {
                elements.push(CodeElement {
                    name,
                    kind: CodeElementKind::Enum,
                    file: filename.to_string(),
                    line: line_num + 1,
                });
            }
        }

        // Trait definitions
        if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
            if let Some(name) = extract_identifier(trimmed, "trait ") {
                elements.push(CodeElement {
                    name,
                    kind: CodeElementKind::Trait,
                    file: filename.to_string(),
                    line: line_num + 1,
                });
            }
        }

        // Impl blocks
        if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
            if let Some(name) = extract_impl_name(trimmed) {
                elements.push(CodeElement {
                    name,
                    kind: CodeElementKind::Impl,
                    file: filename.to_string(),
                    line: line_num + 1,
                });
            }
        }

        // Use statements
        if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
            if let Some(name) = extract_use_path(trimmed) {
                elements.push(CodeElement {
                    name,
                    kind: CodeElementKind::Use,
                    file: filename.to_string(),
                    line: line_num + 1,
                });
            }
        }

        // Module declarations
        if trimmed.starts_with("pub mod ") || trimmed.starts_with("mod ") {
            if let Some(name) = extract_identifier(trimmed, "mod ") {
                elements.push(CodeElement {
                    name,
                    kind: CodeElementKind::Module,
                    file: filename.to_string(),
                    line: line_num + 1,
                });
            }
        }
    }

    elements
}

/// Extract identifier after a keyword like "fn ", "struct ", etc.
fn extract_identifier(line: &str, keyword: &str) -> Option<String> {
    let rest = line.split(keyword).nth(1)?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Extract type name from an impl block.
fn extract_impl_name(line: &str) -> Option<String> {
    // Handle "impl Foo", "impl<T> Foo", "impl Trait for Foo"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        // Check for "for" keyword indicating trait impl
        if let Some(for_idx) = parts.iter().position(|&p| p == "for") {
            if for_idx + 1 < parts.len() {
                let name: String = parts[for_idx + 1]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                return if name.is_empty() { None } else { Some(name) };
            }
        }
        // Simple impl: "impl Foo"
        let type_part = parts[1];
        let name: String = type_part
            .chars()
            .skip_while(|c| *c == '<' || *c == '>')
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

/// Extract the last segment of a use path.
fn extract_use_path(line: &str) -> Option<String> {
    let rest = line.split("use ").nth(1)?;
    let path = rest.trim_end_matches(';').trim();
    let last = path.rsplit("::").next()?;
    let name: String = last
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() || name == "*" {
        None
    } else {
        Some(name)
    }
}

/// Generate a document string from code elements for colony ingestion.
pub fn elements_to_document(elements: &[CodeElement], filename: &str) -> String {
    let mut doc = format!("Source file: {}. ", filename);
    for elem in elements {
        doc.push_str(&format!(
            "{} {} defined at line {}. ",
            elem.kind.as_str(),
            elem.name,
            elem.line
        ));
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_functions() {
        let source = "pub fn hello_world() {\n}\nfn private_fn() {}";
        let elements = extract_code_elements(source, "test.rs");
        let fns: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Function)
            .collect();
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "hello_world");
    }

    #[test]
    fn extract_structs_and_enums() {
        let source = "pub struct Foo {}\nenum Bar {}";
        let elements = extract_code_elements(source, "test.rs");
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn extract_impl_blocks() {
        let source = "impl Foo {\n}\nimpl Display for Bar {}\nimpl<T> Clone for Baz<T> {}";
        let elements = extract_code_elements(source, "test.rs");
        let impls: Vec<_> = elements
            .iter()
            .filter(|e| e.kind == CodeElementKind::Impl)
            .collect();
        assert!(impls.len() >= 2);
    }
}
