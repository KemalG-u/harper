use harper_core::parsers::{self, Markdown, MarkdownOptions, Parser};
use harper_core::{Masker, Token, TokenKind};
use harper_tree_sitter::TreeSitterMasker;
use tree_sitter::Node;

pub struct GitCommitParser {
    inner: parsers::Mask<TreeSitterMasker, Markdown>,
    prefixes: TreeSitterMasker,
}

impl GitCommitParser {
    fn node_condition(n: &Node) -> bool {
        matches!(n.kind(), "subject" | "message_line" | "breaking_change")
    }

    pub fn new(markdown_options: MarkdownOptions) -> Self {
        Self {
            inner: parsers::Mask::new(
                TreeSitterMasker::new(tree_sitter_gitcommit::LANGUAGE.into(), Self::node_condition),
                Markdown::new(markdown_options),
            ),
            prefixes: TreeSitterMasker::new(tree_sitter_gitcommit::LANGUAGE.into(), |node| {
                node.kind() == "prefix"
                    && node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "subject")
            }),
        }
    }
}

impl Default for GitCommitParser {
    fn default() -> Self {
        Self::new(MarkdownOptions::default())
    }
}

impl Parser for GitCommitParser {
    fn parse(&self, source: &[char]) -> Vec<Token> {
        let prefix_mask = self.prefixes.create_mask(source);
        let mut tokens = if let Some((mut prefix, _)) = prefix_mask.iter_allowed(source).next() {
            while prefix.end < source.len() && matches!(source[prefix.end], ' ' | '\t') {
                prefix.end += 1;
            }

            let stripped = [&source[..prefix.start], &source[prefix.end..]].concat();
            let mut tokens = self.inner.parse(&stripped);
            let removed = prefix.end - prefix.start;

            // Keep all token spans in the original, character-indexed source.
            for token in &mut tokens {
                if token.span.start >= prefix.start {
                    token.span.start += removed;
                }
                if token.span.end > prefix.start {
                    token.span.end += removed;
                }
            }

            tokens
        } else {
            self.inner.parse(source)
        };

        for token in &mut tokens {
            if let TokenKind::Space(v) = &mut token.kind {
                *v = (*v).clamp(0, 1);
            }
        }

        tokens
    }
}
