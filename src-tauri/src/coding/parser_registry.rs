//! Parser registry for multi-language Tree-sitter support (Chunk 37.3).
//!
//! Provides a unified interface for creating parsers and extracting symbols
//! from source files in any supported language. New languages are added by:
//! 1. Adding the tree-sitter grammar crate as an optional dep in Cargo.toml.
//! 2. Implementing a feature-gated `extract_{lang}_symbols` function.
//! 3. Registering the language in `SUPPORTED_LANGUAGES`.

use crate::coding::symbol_index::{CodeEdge, Symbol};
#[cfg(any(
    feature = "parser-python",
    feature = "parser-go",
    feature = "parser-java",
    feature = "parser-c"
))]
use crate::coding::symbol_index::{EdgeKind, SymbolKind};

/// A supported source language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    TypeScript,
    #[cfg(feature = "parser-python")]
    Python,
    #[cfg(feature = "parser-go")]
    Go,
    #[cfg(feature = "parser-java")]
    Java,
    #[cfg(feature = "parser-c")]
    C,
    #[cfg(feature = "parser-c")]
    Cpp,
}

impl Language {
    /// File extensions that map to this language.
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Rust => &["rs"],
            Self::TypeScript => &["ts", "tsx"],
            #[cfg(feature = "parser-python")]
            Self::Python => &["py"],
            #[cfg(feature = "parser-go")]
            Self::Go => &["go"],
            #[cfg(feature = "parser-java")]
            Self::Java => &["java"],
            #[cfg(feature = "parser-c")]
            Self::C => &["c", "h"],
            #[cfg(feature = "parser-c")]
            Self::Cpp => &["cpp", "cxx", "cc", "hpp", "hxx"],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            #[cfg(feature = "parser-python")]
            Self::Python => "python",
            #[cfg(feature = "parser-go")]
            Self::Go => "go",
            #[cfg(feature = "parser-java")]
            Self::Java => "java",
            #[cfg(feature = "parser-c")]
            Self::C => "c",
            #[cfg(feature = "parser-c")]
            Self::Cpp => "cpp",
        }
    }
}

/// All currently compiled languages.
pub fn supported_languages() -> Vec<Language> {
    #[allow(unused_mut)]
    let mut langs = vec![Language::Rust, Language::TypeScript];
    #[cfg(feature = "parser-python")]
    langs.push(Language::Python);
    #[cfg(feature = "parser-go")]
    langs.push(Language::Go);
    #[cfg(feature = "parser-java")]
    langs.push(Language::Java);
    #[cfg(feature = "parser-c")]
    {
        langs.push(Language::C);
        langs.push(Language::Cpp);
    }
    langs
}

/// All file extensions we can parse.
pub fn supported_extensions() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut exts = vec!["rs", "ts", "tsx"];
    #[cfg(feature = "parser-python")]
    exts.push("py");
    #[cfg(feature = "parser-go")]
    exts.push("go");
    #[cfg(feature = "parser-java")]
    exts.push("java");
    #[cfg(feature = "parser-c")]
    exts.extend_from_slice(&["c", "h", "cpp", "cxx", "cc", "hpp", "hxx"]);
    exts
}

/// Detect the language for a file extension.
pub fn detect_language(ext: &str) -> Option<Language> {
    supported_languages()
        .into_iter()
        .find(|lang| lang.extensions().contains(&ext))
}

/// Create a tree-sitter parser for the given language.
pub fn create_parser(lang: Language) -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    let grammar: tree_sitter::Language = match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        #[cfg(feature = "parser-python")]
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        #[cfg(feature = "parser-go")]
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        #[cfg(feature = "parser-java")]
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        #[cfg(feature = "parser-c")]
        Language::C => tree_sitter_c::LANGUAGE.into(),
        #[cfg(feature = "parser-c")]
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
    };
    parser
        .set_language(&grammar)
        .unwrap_or_else(|_| panic!("failed to load tree-sitter-{} grammar", lang.name()));
    parser
}

/// Extract symbols and edges from parsed source in the given language.
pub fn extract_symbols(
    lang: Language,
    source: &str,
    root: tree_sitter::Node,
    file: &str,
) -> (Vec<Symbol>, Vec<CodeEdge>) {
    match lang {
        Language::Rust | Language::TypeScript => {
            // These are handled by the existing symbol_index.rs extractors
            // and should not go through this path. Provided for completeness.
            let _ = (source, root, file);
            (Vec::new(), Vec::new())
        }
        #[cfg(feature = "parser-python")]
        Language::Python => extract_python_symbols(source, root, file),
        #[cfg(feature = "parser-go")]
        Language::Go => extract_go_symbols(source, root, file),
        #[cfg(feature = "parser-java")]
        Language::Java => extract_java_symbols(source, root, file),
        #[cfg(feature = "parser-c")]
        Language::C | Language::Cpp => extract_c_symbols(source, root, file),
    }
}

// ─── Helper ─────────────────────────────────────────────────────────────────

#[cfg(any(
    feature = "parser-python",
    feature = "parser-go",
    feature = "parser-java",
    feature = "parser-c"
))]
fn node_text<'a>(source: &'a str, node: &tree_sitter::Node) -> &'a str {
    &source[node.byte_range()]
}

#[cfg(any(
    feature = "parser-python",
    feature = "parser-go",
    feature = "parser-java",
    feature = "parser-c"
))]
fn find_child_by_field<'a>(
    node: &tree_sitter::Node<'a>,
    field: &str,
) -> Option<tree_sitter::Node<'a>> {
    node.child_by_field_name(field)
}

#[cfg(any(
    feature = "parser-python",
    feature = "parser-go",
    feature = "parser-java",
    feature = "parser-c"
))]
fn node_name_text(source: &str, node: &tree_sitter::Node, field: &str) -> Option<String> {
    find_child_by_field(node, field).map(|n| node_text(source, &n).to_string())
}

// ─── Python extraction ──────────────────────────────────────────────────────

#[cfg(feature = "parser-python")]
fn extract_python_symbols(
    source: &str,
    root: tree_sitter::Node,
    file: &str,
) -> (Vec<Symbol>, Vec<CodeEdge>) {
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    walk_python_node(source, root, file, None, &mut symbols, &mut edges);
    (symbols, edges)
}

#[cfg(feature = "parser-python")]
fn walk_python_node(
    source: &str,
    node: tree_sitter::Node,
    file: &str,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
    edges: &mut Vec<CodeEdge>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                let kind = if parent_name.is_some() {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                };
                symbols.push(Symbol {
                    name: name.clone(),
                    kind,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
                // Recurse into body for nested definitions
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_python_node(source, child, file, Some(&name), symbols, edges);
                }
                return;
            }
        }
        "class_definition" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_python_node(source, child, file, Some(&name), symbols, edges);
                }
                return;
            }
        }
        "import_statement" | "import_from_statement" => {
            // Extract import names
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
                    let target = if child.kind() == "aliased_import" {
                        find_child_by_field(&child, "name")
                            .map(|n| node_text(source, &n).to_string())
                    } else {
                        Some(node_text(source, &child).to_string())
                    };
                    if let Some(name) = target {
                        edges.push(CodeEdge::from_node(file, &node, EdgeKind::Imports, name));
                    }
                }
            }
        }
        "call" => {
            if let Some(func_node) = find_child_by_field(&node, "function") {
                let name = node_text(source, &func_node).to_string();
                // Only track simple names, not complex expressions
                if !name.contains('\n') && name.len() < 80 {
                    edges.push(CodeEdge::from_node(file, &node, EdgeKind::Calls, name));
                }
            }
        }
        _ => {}
    }
    // Default: recurse into children.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_python_node(source, child, file, parent_name, symbols, edges);
    }
}

// ─── Go extraction ──────────────────────────────────────────────────────────

#[cfg(feature = "parser-go")]
fn extract_go_symbols(
    source: &str,
    root: tree_sitter::Node,
    file: &str,
) -> (Vec<Symbol>, Vec<CodeEdge>) {
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    walk_go_node(source, root, file, &mut symbols, &mut edges);
    (symbols, edges)
}

#[cfg(feature = "parser-go")]
fn walk_go_node(
    source: &str,
    node: tree_sitter::Node,
    file: &str,
    symbols: &mut Vec<Symbol>,
    edges: &mut Vec<CodeEdge>,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Function,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: None,
                });
            }
        }
        "method_declaration" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                // Try to find receiver type
                let receiver = find_child_by_field(&node, "receiver").and_then(|r| {
                    let mut cursor = r.walk();
                    r.children(&mut cursor)
                        .find(|c| c.kind() == "type_identifier")
                        .map(|t| node_text(source, &t).to_string())
                });
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Method,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: receiver,
                });
            }
        }
        "type_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_spec" {
                    if let Some(name) = node_name_text(source, &child, "name") {
                        // Determine kind from the type body
                        let kind = child
                            .child_by_field_name("type")
                            .map(|t| match t.kind() {
                                "struct_type" => SymbolKind::Struct,
                                "interface_type" => SymbolKind::Interface,
                                _ => SymbolKind::TypeAlias,
                            })
                            .unwrap_or(SymbolKind::TypeAlias);
                        symbols.push(Symbol {
                            name,
                            kind,
                            file: file.to_string(),
                            line: child.start_position().row as u32 + 1,
                            end_line: child.end_position().row as u32 + 1,
                            parent: None,
                        });
                    }
                }
            }
        }
        "import_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "import_spec" || child.kind() == "import_spec_list" {
                    let mut inner_cursor = child.walk();
                    for spec in child.children(&mut inner_cursor) {
                        if spec.kind() == "import_spec"
                            || spec.kind() == "interpreted_string_literal"
                        {
                            let text = node_text(source, &spec).trim_matches('"').to_string();
                            if !text.is_empty() {
                                edges.push(CodeEdge::from_node(
                                    file,
                                    &spec,
                                    EdgeKind::Imports,
                                    text,
                                ));
                            }
                        }
                    }
                }
                if child.kind() == "interpreted_string_literal" {
                    let text = node_text(source, &child).trim_matches('"').to_string();
                    if !text.is_empty() {
                        edges.push(CodeEdge::from_node(file, &child, EdgeKind::Imports, text));
                    }
                }
            }
        }
        "call_expression" => {
            if let Some(func_node) = find_child_by_field(&node, "function") {
                let name = node_text(source, &func_node).to_string();
                if !name.contains('\n') && name.len() < 80 {
                    edges.push(CodeEdge::from_node(file, &node, EdgeKind::Calls, name));
                }
            }
        }
        "const_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "const_spec" {
                    if let Some(name) = node_name_text(source, &child, "name") {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Constant,
                            file: file.to_string(),
                            line: child.start_position().row as u32 + 1,
                            end_line: child.end_position().row as u32 + 1,
                            parent: None,
                        });
                    }
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_go_node(source, child, file, symbols, edges);
    }
}

// ─── Java extraction ────────────────────────────────────────────────────────

#[cfg(feature = "parser-java")]
fn extract_java_symbols(
    source: &str,
    root: tree_sitter::Node,
    file: &str,
) -> (Vec<Symbol>, Vec<CodeEdge>) {
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    walk_java_node(source, root, file, None, &mut symbols, &mut edges);
    (symbols, edges)
}

#[cfg(feature = "parser-java")]
fn walk_java_node(
    source: &str,
    node: tree_sitter::Node,
    file: &str,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
    edges: &mut Vec<CodeEdge>,
) {
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_java_node(source, child, file, Some(&name), symbols, edges);
                }
                return;
            }
        }
        "interface_declaration" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_java_node(source, child, file, Some(&name), symbols, edges);
                }
                return;
            }
        }
        "enum_declaration" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Enum,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_java_node(source, child, file, Some(&name), symbols, edges);
                }
                return;
            }
        }
        "method_declaration" | "constructor_declaration" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Method,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
            }
        }
        "import_declaration" => {
            // `import foo.bar.Baz;`
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "scoped_identifier" {
                    let text = node_text(source, &child).to_string();
                    edges.push(CodeEdge::from_node(file, &node, EdgeKind::Imports, text));
                }
            }
        }
        "method_invocation" => {
            if let Some(name_node) = find_child_by_field(&node, "name") {
                let name = node_text(source, &name_node).to_string();
                edges.push(CodeEdge::from_node(file, &node, EdgeKind::Calls, name));
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_java_node(source, child, file, parent_name, symbols, edges);
    }
}

// ─── C/C++ extraction ───────────────────────────────────────────────────────

#[cfg(feature = "parser-c")]
fn extract_c_symbols(
    source: &str,
    root: tree_sitter::Node,
    file: &str,
) -> (Vec<Symbol>, Vec<CodeEdge>) {
    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    walk_c_node(source, root, file, None, &mut symbols, &mut edges);
    (symbols, edges)
}

#[cfg(feature = "parser-c")]
fn walk_c_node(
    source: &str,
    node: tree_sitter::Node,
    file: &str,
    parent_name: Option<&str>,
    symbols: &mut Vec<Symbol>,
    edges: &mut Vec<CodeEdge>,
) {
    match node.kind() {
        "function_definition" => {
            // Declarator contains the function name
            if let Some(declarator) = find_child_by_field(&node, "declarator") {
                if let Some(name) = extract_c_declarator_name(source, &declarator) {
                    symbols.push(Symbol {
                        name,
                        kind: SymbolKind::Function,
                        file: file.to_string(),
                        line: node.start_position().row as u32 + 1,
                        end_line: node.end_position().row as u32 + 1,
                        parent: parent_name.map(|s| s.to_string()),
                    });
                }
            }
        }
        "struct_specifier" | "union_specifier" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Struct,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
            }
        }
        "enum_specifier" => {
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name,
                    kind: SymbolKind::Enum,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
            }
        }
        "class_specifier" => {
            // C++ class
            if let Some(name) = node_name_text(source, &node, "name") {
                symbols.push(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    file: file.to_string(),
                    line: node.start_position().row as u32 + 1,
                    end_line: node.end_position().row as u32 + 1,
                    parent: parent_name.map(|s| s.to_string()),
                });
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_c_node(source, child, file, Some(&name), symbols, edges);
                }
                return;
            }
        }
        "preproc_include" => {
            if let Some(path_node) = find_child_by_field(&node, "path") {
                let text = node_text(source, &path_node)
                    .trim_matches(|c| c == '"' || c == '<' || c == '>')
                    .to_string();
                if !text.is_empty() {
                    edges.push(CodeEdge::from_node(file, &node, EdgeKind::Imports, text));
                }
            }
        }
        "call_expression" => {
            if let Some(func_node) = find_child_by_field(&node, "function") {
                let name = node_text(source, &func_node).to_string();
                if !name.contains('\n') && name.len() < 80 {
                    edges.push(CodeEdge::from_node(file, &node, EdgeKind::Calls, name));
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_c_node(source, child, file, parent_name, symbols, edges);
    }
}

#[cfg(feature = "parser-c")]
fn extract_c_declarator_name(source: &str, node: &tree_sitter::Node) -> Option<String> {
    // C function declarators can be nested: `int (*foo)(int)` or `int foo(int)`
    // Walk down to find the identifier.
    match node.kind() {
        "identifier" | "field_identifier" => Some(node_text(source, node).to_string()),
        "function_declarator" | "pointer_declarator" | "parenthesized_declarator" => {
            find_child_by_field(node, "declarator")
                .and_then(|d| extract_c_declarator_name(source, &d))
        }
        _ => {
            // Try first named child as fallback
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.is_named() {
                    if let Some(name) = extract_c_declarator_name(source, &child) {
                        return Some(name);
                    }
                }
            }
            None
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_default() {
        assert_eq!(detect_language("rs"), Some(Language::Rust));
        assert_eq!(detect_language("ts"), Some(Language::TypeScript));
        assert_eq!(detect_language("tsx"), Some(Language::TypeScript));
        assert_eq!(detect_language("txt"), None);
    }

    #[test]
    fn test_supported_extensions_includes_core() {
        let exts = supported_extensions();
        assert!(exts.contains(&"rs"));
        assert!(exts.contains(&"ts"));
        assert!(exts.contains(&"tsx"));
    }

    #[cfg(feature = "parser-python")]
    #[test]
    fn test_python_extraction() {
        let source = r#"
import os
from pathlib import Path

class Server:
    def __init__(self, port):
        self.port = port

    def start(self):
        os.system("serve")

def main():
    s = Server(8080)
    s.start()
"#;
        let mut parser = create_parser(Language::Python);
        let tree = parser.parse(source, None).unwrap();
        let (symbols, edges) = extract_python_symbols(source, tree.root_node(), "app.py");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Server"));
        assert!(names.contains(&"__init__"));
        assert!(names.contains(&"start"));
        assert!(names.contains(&"main"));

        assert!(edges
            .iter()
            .any(|e| e.kind == EdgeKind::Imports && e.target_name == "os"));
    }

    #[cfg(feature = "parser-go")]
    #[test]
    fn test_go_extraction() {
        let source = r#"
package main

import "fmt"

type Server struct {
    Port int
}

func (s *Server) Start() {
    fmt.Println("starting")
}

func main() {
    s := &Server{Port: 8080}
    s.Start()
}
"#;
        let mut parser = create_parser(Language::Go);
        let tree = parser.parse(source, None).unwrap();
        let (symbols, edges) = extract_go_symbols(source, tree.root_node(), "main.go");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Server"));
        assert!(names.contains(&"Start"));
        assert!(names.contains(&"main"));

        assert!(edges
            .iter()
            .any(|e| e.kind == EdgeKind::Imports && e.target_name == "fmt"));
    }

    #[cfg(feature = "parser-java")]
    #[test]
    fn test_java_extraction() {
        let source = r#"
import java.util.List;

public class Server {
    private int port;

    public Server(int port) {
        this.port = port;
    }

    public void start() {
        System.out.println("starting");
    }
}
"#;
        let mut parser = create_parser(Language::Java);
        let tree = parser.parse(source, None).unwrap();
        let (symbols, edges) = extract_java_symbols(source, tree.root_node(), "Server.java");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Server"));
        assert!(names.contains(&"start"));

        assert!(edges
            .iter()
            .any(|e| e.kind == EdgeKind::Imports && e.target_name.contains("java.util.List")));
    }

    #[cfg(feature = "parser-c")]
    #[test]
    fn test_c_extraction() {
        let source = r#"
#include <stdio.h>
#include "mylib.h"

struct Point {
    int x;
    int y;
};

enum Color { RED, GREEN, BLUE };

void print_point(struct Point p) {
    printf("(%d, %d)", p.x, p.y);
}

int main() {
    struct Point p = {1, 2};
    print_point(p);
    return 0;
}
"#;
        let mut parser = create_parser(Language::C);
        let tree = parser.parse(source, None).unwrap();
        let (symbols, edges) = extract_c_symbols(source, tree.root_node(), "main.c");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Point"));
        assert!(names.contains(&"Color"));
        assert!(names.contains(&"print_point"));
        assert!(names.contains(&"main"));

        assert!(edges
            .iter()
            .any(|e| e.kind == EdgeKind::Imports && e.target_name == "stdio.h"));
        assert!(edges
            .iter()
            .any(|e| e.kind == EdgeKind::Imports && e.target_name == "mylib.h"));
        assert!(edges
            .iter()
            .any(|e| e.kind == EdgeKind::Calls && e.target_name == "printf"));
    }
}
