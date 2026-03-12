#![doc = include_str!("../README.md")]

use rushdown::as_kind_data;
use rushdown::as_type_data;
use rushdown::ast::Arena;
use rushdown::ast::CodeBlock;
use rushdown::ast::NodeRef;
use rushdown::ast::WalkStatus;
use rushdown::renderer;
use rushdown::renderer::html;
use rushdown::renderer::html::Renderer;
use rushdown::renderer::html::RendererExtension;
use rushdown::renderer::html::RendererExtensionFn;
use rushdown::renderer::BoxRenderNode;
use rushdown::renderer::NodeRenderer;
use rushdown::renderer::NodeRendererRegistry;
use rushdown::renderer::RenderNode;
use rushdown::renderer::RendererOptions;
use rushdown::renderer::TextWrite;
use rushdown::Result;
use std::any::TypeId;
use std::rc::Rc;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::css_for_theme_with_class_style;
use syntect::html::styled_line_to_highlighted_html;
use syntect::html::ClassStyle;
use syntect::html::ClassedHTMLGenerator;
use syntect::html::IncludeBackground;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

// Renderer {{{

/// Options for the `HighlightingHtmlRenderer`.
#[derive(Debug, Clone)]
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
}

impl Default for HighlightingHtmlRendererOptions {
    fn default() -> Self {
        Self {
            theme: "InspiredGitHub",
            mode: HighlightingMode::default(),
            theme_set: None,
        }
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

#[allow(dead_code)]
struct HighlightingHtmlRenderer<W: TextWrite> {
    _phantom: core::marker::PhantomData<W>,
    writer: html::Writer,
    options: HighlightingHtmlRendererOptions,
    syntax_set: SyntaxSet,
    default_theme_set: ThemeSet,
}

impl<W: TextWrite> HighlightingHtmlRenderer<W> {
    fn with_options(options: HighlightingHtmlRendererOptions, html_opts: html::Options) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            writer: html::Writer::with_options(html_opts),
            syntax_set: SyntaxSet::load_defaults_newlines(),
            default_theme_set: ThemeSet::load_defaults(),
            options,
        }
    }

    fn render_code_to_html_attr(
        &self,
        language: &str,
        code: &str,
        theme_name: &str,
    ) -> Option<String> {
        let ps = &self.syntax_set;
        let ts = if let Some(ref theme_set) = self.options.theme_set {
            theme_set
        } else {
            &self.default_theme_set
        };

        let theme = ts
            .themes
            .get(theme_name)
            .unwrap_or_else(|| &ts.themes["InspiredGitHub"]);

        let lang = if language.is_empty() {
            "plaintext"
        } else {
            language
        };
        let syntax = ps
            .find_syntax_by_token(lang)
            .or_else(|| ps.find_syntax_by_extension(lang))
            .unwrap_or_else(|| ps.find_syntax_plain_text());

        let bg = theme
            .settings
            .background
            .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
            .unwrap_or_else(|| "#ffffff".to_string());

        let mut out = String::new();
        out.push_str(&format!(
        r#"<pre style="background-color: {}; padding: 12px; overflow: auto;"><code class="language-{}">"#,
        bg, language
    ));

        let mut h = HighlightLines::new(syntax, theme);

        for line in LinesWithEndings::from(code) {
            let regions = h.highlight_line(line, ps).ok()?;
            let html_line =
                styled_line_to_highlighted_html(&regions[..], IncludeBackground::No).ok()?;
            out.push_str(&html_line);
        }

        out.push_str("</code></pre>\n");
        Some(out)
    }

    fn render_with_classes(&self, language: &str, code: &str) -> Option<String> {
        let ps = &self.syntax_set;

        let lang = if language.is_empty() {
            "plaintext"
        } else {
            language
        };

        let syntax = ps
            .find_syntax_by_token(lang)
            .or_else(|| ps.find_syntax_by_extension(lang))
            .unwrap_or_else(|| ps.find_syntax_plain_text());

        let mut html_gen =
            ClassedHTMLGenerator::new_with_class_style(syntax, ps, ClassStyle::Spaced);

        let mut html = String::new();
        html.push_str(&format!(
            r#"<pre class="code"><code class="language-{}">"#,
            language
        ));
        for line in LinesWithEndings::from(code) {
            html_gen
                .parse_html_for_line_which_includes_newline(line)
                .ok()?;
        }
        html.push_str(&html_gen.finalize());
        html.push_str("</code></pre>\n");
        Some(html)
    }
}

impl<W: TextWrite> RenderNode<W> for HighlightingHtmlRenderer<W> {
    /// Renders a paragraph node.
    fn render_node<'a>(
        &self,
        w: &mut W,
        source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _ctx: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        if entering {
            let kd = as_kind_data!(arena, node_ref, CodeBlock);
            let block = as_type_data!(arena, node_ref, Block);
            let mut code = String::new();
            for line in block.lines().iter() {
                code.push_str(&line.str(source));
            }
            let lang = kd.language(source).unwrap_or("plaintext");
            match self.options.mode {
                HighlightingMode::Attribute => {
                    if let Some(html) =
                        self.render_code_to_html_attr(lang, &code, self.options.theme)
                    {
                        w.write_str(&html)?;
                        return Ok(WalkStatus::Continue);
                    }
                }
                HighlightingMode::Class => {
                    if let Some(html) = self.render_with_classes(lang, &code) {
                        w.write_str(&html)?;
                        return Ok(WalkStatus::Continue);
                    }
                }
            }

            self.writer.write_safe_str(w, "<pre><code")?;
            if let Some(lang) = kd.language(source) {
                self.writer.write_safe_str(w, " class=\"language-")?;
                self.writer.write(w, lang)?;
                self.writer.write_safe_str(w, "\"")?;
            }
            self.writer.write_safe_str(w, ">")?;
            let block = as_type_data!(arena, node_ref, Block);
            for line in block.lines().iter() {
                self.writer.raw_write(w, &line.str(source))?;
            }
            self.writer.write_safe_str(w, "</code></pre>\n")?;
        }
        Ok(WalkStatus::Continue)
    }
}

impl<'r, W> NodeRenderer<'r, W> for HighlightingHtmlRenderer<W>
where
    W: TextWrite + 'r,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'r, W>) {
        nrr.register_node_renderer_fn(TypeId::of::<CodeBlock>(), BoxRenderNode::new(self));
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
    RendererExtensionFn::new(move |r: &mut Renderer<'cb, W>| {
        let options = options.into();
        r.add_node_renderer(HighlightingHtmlRenderer::with_options, options);
    })
}

// }}}
