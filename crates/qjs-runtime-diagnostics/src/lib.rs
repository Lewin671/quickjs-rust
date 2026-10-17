//! Compile-time overlays for `qjs-runtime`'s opt-in diagnostic feature.
//!
//! Production source positions are build inputs: even erased counter statements
//! move panic locations and change the release image. Disabled instrumentation
//! therefore loads the original modules. Enabled instrumentation expands strict,
//! checked source patches into the same module tree, without a runtime layer.
extern crate proc_macro;

use proc_macro::{Delimiter, Group, Literal, TokenStream, TokenTree};
use std::{
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};
mod patch;

const PATCHES: &[&str] = &[
    include_str!("../patches/counters.patch"),
    include_str!("../patches/ownership.patch"),
    include_str!("../patches/properties.patch"),
    include_str!("../patches/execution.patch"),
];

/// Expands a feature-only module body from production source and checked hooks.
#[proc_macro]
pub fn instrument_body(input: TokenStream) -> TokenStream {
    let result = (|| {
        let tokens: Vec<_> = input.into_iter().collect();
        let [TokenTree::Literal(path)] = tokens.as_slice() else {
            return Err("expected \"src/file.rs\"".to_owned());
        };
        let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
        let file = root.join(string_literal(path)?);
        let mut body: Vec<_> = expand_file(&root, &file)?.into_iter().collect();
        // A module's inner docs belong to the original file. Attribute output
        // is an item fragment, so do not emit inner docs at this macro site.
        while body.len() >= 3 && punct(&body[0], '#') && punct(&body[1], '!') {
            let TokenTree::Group(attribute) = &body[2] else {
                break;
            };
            if !attribute
                .stream()
                .into_iter()
                .next()
                .is_some_and(|token| ident(&token, "doc"))
            {
                return Err("diagnostic entry has a non-documentation inner attribute".to_owned());
            }
            body.drain(..3);
        }
        Ok(body.into_iter().collect())
    })();
    result.unwrap_or_else(|error: String| {
        TokenStream::from_str(&format!("compile_error!({error:?});"))
            .expect("quoted compiler error")
    })
}

fn string_literal(literal: &Literal) -> Result<String, String> {
    let text = literal.to_string();
    // Repository module paths are plain ASCII literals, never JS input.
    text.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .filter(|s| !s.contains('\\'))
        .map(str::to_owned)
        .ok_or_else(|| format!("expected plain path literal: {text}"))
}
fn ident(token: &TokenTree, name: &str) -> bool {
    matches!(token,TokenTree::Ident(value) if value.to_string()==name)
}
fn punct(token: &TokenTree, value: char) -> bool {
    matches!(token,TokenTree::Punct(p) if p.as_char()==value)
}

fn expand_file(root: &Path, file: &Path) -> Result<TokenStream, String> {
    let source = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let relative = file
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("non-UTF8 module path")?
        .replace('\\', "/");
    let source = patch::apply(&relative, &source, PATCHES)?;
    let stream = TokenStream::from_str(&source).map_err(|e| format!("{relative}: {e}"))?;
    let parent = file.parent().ok_or("module parent")?;
    let stem = file
        .file_stem()
        .and_then(|p| p.to_str())
        .ok_or("module stem")?;
    let directory = if matches!(stem, "lib" | "mod") {
        parent.to_owned()
    } else {
        parent.join(stem)
    };
    let stream = expand_modules(root, &directory, parent, stream)?;
    // Stable include_str! dependency tracking: editing any original module must
    // rebuild the consumer, even though the tokens came from a proc macro.
    let mut tokens: Vec<_> = stream.into_iter().collect();
    let mut at = 0;
    while tokens.len() >= at + 3
        && punct(&tokens[at], '#')
        && punct(&tokens[at + 1], '!')
        && matches!(&tokens[at+2],TokenTree::Group(g) if g.delimiter()==Delimiter::Bracket)
    {
        at += 3;
    }
    let tracking = TokenStream::from_str(&format!(
        "const _: &str = include_str!({:?});",
        file.display().to_string()
    ))
    .map_err(|e| e.to_string())?;
    tokens.splice(at..at, tracking);
    Ok(tokens.into_iter().collect())
}

fn take_path(output: &mut Vec<TokenTree>) -> Result<Option<String>, String> {
    let mut at = output.len();
    let mut found = None;
    while at >= 2 && punct(&output[at - 2], '#') {
        let TokenTree::Group(group) = &output[at - 1] else {
            break;
        };
        if group.delimiter() != Delimiter::Bracket {
            break;
        }
        let attribute: Vec<_> = group.stream().into_iter().collect();
        if let [key, eq, TokenTree::Literal(value)] = attribute.as_slice()
            && ident(key, "path")
            && punct(eq, '=')
        {
            found = Some(string_literal(value)?);
            output.drain(at - 2..at);
        }
        at -= 2;
    }
    Ok(found)
}

fn expand_modules(
    root: &Path,
    directory: &Path,
    path_base: &Path,
    stream: TokenStream,
) -> Result<TokenStream, String> {
    let input: Vec<_> = stream.into_iter().collect();
    let mut output = Vec::new();
    let mut at = 0;
    while at < input.len() {
        if at + 2 < input.len() && ident(&input[at], "mod") {
            if let TokenTree::Ident(name) = &input[at + 1] {
                if punct(&input[at + 2], ';') {
                    let explicit = take_path(&mut output)?;
                    let child = if let Some(path) = explicit {
                        path_base.join(path)
                    } else {
                        let direct = directory.join(format!("{name}.rs"));
                        if direct.is_file() {
                            direct
                        } else {
                            directory.join(format!("{name}/mod.rs"))
                        }
                    };
                    output.push(input[at].clone());
                    output.push(input[at + 1].clone());
                    output.push(TokenTree::Group(Group::new(
                        Delimiter::Brace,
                        expand_file(root, &child)?,
                    )));
                    at += 3;
                    continue;
                }
                if let TokenTree::Group(group) = &input[at + 2]
                    && group.delimiter() == Delimiter::Brace
                {
                    let child = directory.join(name.to_string());
                    let body = expand_modules(root, &child, &child, group.stream())?;
                    output.push(input[at].clone());
                    output.push(input[at + 1].clone());
                    output.push(TokenTree::Group(Group::new(Delimiter::Brace, body)));
                    at += 3;
                    continue;
                }
            }
        }
        if let TokenTree::Group(group) = &input[at] {
            let body = expand_modules(root, directory, path_base, group.stream())?;
            output.push(TokenTree::Group(Group::new(group.delimiter(), body)));
        } else {
            output.push(input[at].clone());
        }
        at += 1;
    }
    Ok(output.into_iter().collect())
}
