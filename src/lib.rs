#![doc = include_str!("../README.md")]

use core::any::TypeId;
use core::fmt;
use core::marker::PhantomData;
use std::rc::Rc;

use rushdown::{
    as_kind_data,
    ast::{Arena, CodeBlock, CodeBlockKind, NodeRef, WalkStatus},
    renderer::{
        self,
        html::{self, renderer_extension, RendererExtension},
        BoxRenderNodeDecorator, NextRenderNode, NodeRenderer, NodeRendererRegistry,
        RenderNodeDecorator, RendererOptions, TextWrite,
    },
    Result,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::css_for_theme_with_class_style;
use syntect::html::styled_line_to_highlighted_html;
use syntect::html::ClassStyle;
use syntect::html::ClassedHTMLGenerator;
use syntect::html::IncludeBackground;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

// Renderer {{{

/// A function that decides whether a code block should be excluded from highlighting.
pub type ExcludeFn = Rc<dyn Fn(&CodeBlock, &str, &renderer::Context) -> bool>;

/// Options for the `HighlightingHtmlRenderer`.
#[derive(Clone)]
pub struct HighlightingHtmlRendererOptions {
    /// The name of the syntax highlighting theme to use.
    /// This value is only used if the `mode` is set to `HighlightingMode::Attribute`. If the theme
    /// is not found, it falls back to "InspiredGitHub".
    pub theme: &'static str,

    /// The mode to use for syntax highlighting. This determines how the HTML output is structured.
    pub mode: HighlightingMode,

    /// An optional `ThemeSet` to use for syntax highlighting. If not provided, the default themes
    /// will be used.
    pub theme_set: Option<Rc<ThemeSet>>,

    /// An optional function that decides whether a code block should be excluded from
    /// highlighting. Excluded code blocks are rendered by the default renderer.
    pub exclude_fn: Option<ExcludeFn>,

    /// Languages to be excluded from highlighting. Excluded code blocks are rendered by the
    /// default renderer.
    pub exclude_languages: &'static [&'static str],
}

impl Default for HighlightingHtmlRendererOptions {
    fn default() -> Self {
        Self {
            theme: "InspiredGitHub",
            mode: HighlightingMode::default(),
            theme_set: None,
            exclude_fn: None,
            exclude_languages: &[],
        }
    }
}

impl fmt::Debug for HighlightingHtmlRendererOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HighlightingHtmlRendererOptions")
            .field("theme", &self.theme)
            .field("mode", &self.mode)
            .field("theme_set", &self.theme_set)
            .field("exclude_fn", &self.exclude_fn.is_some())
            .field("exclude_languages", &self.exclude_languages)
            .finish()
    }
}

/// The mode to use for syntax highlighting. This determines how the HTML output is structured.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum HighlightingMode {
    #[default]
    Attribute,
    Class,
}

/// Generates CSS for the specified theme. This is only necessary if the `mode` in
/// `HighlightingHtmlRendererOptions` is set to `HighlightingMode::Class`.
pub fn generate_css(theme: &str, theme_set: Option<&ThemeSet>) -> Option<String> {
    if let Some(ts) = theme_set {
        let theme = ts.themes.get(theme)?;
        css_for_theme_with_class_style(theme, ClassStyle::Spaced).ok()
    } else {
        let ts = ThemeSet::load_defaults();
        let theme = ts.themes.get(theme)?;
        css_for_theme_with_class_style(theme, ClassStyle::Spaced).ok()
    }
}

impl RendererOptions for HighlightingHtmlRendererOptions {}

struct HighlightingHtmlRenderer<W: TextWrite> {
    _phantom: PhantomData<W>,
    options: HighlightingHtmlRendererOptions,
    syntax_set: SyntaxSet,
    theme_set: Rc<ThemeSet>,
}

impl<W: TextWrite> HighlightingHtmlRenderer<W> {
    fn with_options(options: HighlightingHtmlRendererOptions) -> Self {
        let theme_set = options
            .theme_set
            .clone()
            .unwrap_or_else(|| Rc::new(ThemeSet::load_defaults()));
        Self {
            _phantom: PhantomData,
            options,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set,
        }
    }

    fn syntax(&self, language: &str) -> Option<&SyntaxReference> {
        self.syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
    }

    fn theme(&self) -> &Theme {
        self.theme_set
            .themes
            .get(self.options.theme)
            .unwrap_or_else(|| &self.theme_set.themes["InspiredGitHub"])
    }

    fn background_color(&self) -> String {
        self.theme()
            .settings
            .background
            .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
            .unwrap_or_else(|| "#ffffff".to_string())
    }

    fn highlight_attribute(&self, syntax: &SyntaxReference, code: &str) -> String {
        let mut out = String::new();
        let mut highlighter = HighlightLines::new(syntax, self.theme());
        for line in LinesWithEndings::from(code) {
            if let Ok(regions) = highlighter.highlight_line(line, &self.syntax_set) {
                if let Ok(html) =
                    styled_line_to_highlighted_html(&regions[..], IncludeBackground::No)
                {
                    out.push_str(&html);
                }
            }
        }
        out
    }

    fn highlight_class(&self, syntax: &SyntaxReference, code: &str) -> String {
        let mut generator = ClassedHTMLGenerator::new_with_class_style(
            syntax,
            &self.syntax_set,
            ClassStyle::Spaced,
        );
        for line in LinesWithEndings::from(code) {
            let _ = generator.parse_html_for_line_which_includes_newline(line);
        }
        generator.finalize()
    }

    fn render_highlighted(
        &self,
        w: &mut W,
        source: &str,
        kd: &CodeBlock,
        lang: &str,
        syntax: &SyntaxReference,
    ) -> Result<()> {
        let mut code = String::new();
        for line in kd.value().iter(source) {
            code.push_str(&line);
        }
        match self.options.mode {
            HighlightingMode::Attribute => {
                w.write_str("<pre style=\"background-color: ")?;
                w.write_str(&self.background_color())?;
                w.write_str("; padding: 12px; overflow: auto;\"><code class=\"language-")?;
                html::write_text(w, lang)?;
                w.write_str("\">")?;
                w.write_str(&self.highlight_attribute(syntax, &code))?;
            }
            HighlightingMode::Class => {
                w.write_str("<pre class=\"code\"><code class=\"language-")?;
                html::write_text(w, lang)?;
                w.write_str("\">")?;
                w.write_str(&self.highlight_class(syntax, &code))?;
            }
        }
        w.write_str("</code></pre>\n")
    }
}

impl<W: TextWrite> RenderNodeDecorator<W> for HighlightingHtmlRenderer<W> {
    fn decorate_node<'a>(
        &self,
        next: &NextRenderNode<'_, W>,
        w: &mut W,
        source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        context: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        let kd = as_kind_data!(arena, node_ref, CodeBlock);
        let lang = kd.language(source);
        let lang = lang.str(source);
        let lang = lang.as_ref();
        let excluded = self
            .options
            .exclude_fn
            .as_ref()
            .is_some_and(|f| f(kd, source, context))
            || self.options.exclude_languages.contains(&lang);
        if excluded {
            return next.render_node(w, source, arena, node_ref, entering, context);
        }
        let syntax = if kd.code_block_kind() == CodeBlockKind::Fenced && !lang.is_empty() {
            self.syntax(lang)
        } else {
            None
        };
        let Some(syntax) = syntax else {
            return next.render_node(w, source, arena, node_ref, entering, context);
        };
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        self.render_highlighted(w, source, kd, lang, syntax)?;
        Ok(WalkStatus::SkipChildren)
    }
}

impl<'r, W> NodeRenderer<'r, W> for HighlightingHtmlRenderer<W>
where
    W: TextWrite + 'r,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'r, W>) {
        nrr.register_node_renderer_decorator_fn(
            TypeId::of::<CodeBlock>(),
            BoxRenderNodeDecorator::new(self),
        );
    }
}

// }}}

// Extension {{{

/// Returns a renderer extension that adds support for rendering code blocks with syntax
/// highlighting.
pub fn highlighting_html_renderer_extension<'cb, W>(
    options: impl Into<HighlightingHtmlRendererOptions>,
) -> impl RendererExtension<'cb, W>
where
    W: TextWrite + 'cb,
{
    let options = options.into();
    renderer_extension(move |r| {
        r.add_node_renderer(HighlightingHtmlRenderer::with_options, options);
    })
}

// }}}
