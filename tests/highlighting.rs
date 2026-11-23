use std::rc::Rc;

use rushdown::{
    new_markdown_to_html_string,
    parser::{self},
    renderer::html,
    test::{MarkdownTestCase, MarkdownTestCaseOptions},
};
use rushdown_highlighting::{
    highlighting_html_renderer_extension, HighlightingHtmlRendererOptions, HighlightingMode,
};

fn render_with(source: &str, options: HighlightingHtmlRendererOptions) -> String {
    let markdown_to_html = new_markdown_to_html_string(
        parser::Options::default(),
        html::Options {
            allows_unsafe: true,
            xhtml: false,
            ..html::Options::default()
        },
        parser::NO_EXTENSIONS,
        highlighting_html_renderer_extension(options),
    );
    let mut output = String::new();
    markdown_to_html(&mut output, source).expect("rendering failed");
    output
}

fn render(source: &str, mode: HighlightingMode) -> String {
    render_with(
        source,
        HighlightingHtmlRendererOptions {
            mode,
            ..HighlightingHtmlRendererOptions::default()
        },
    )
}

#[test]
fn test_highlighting() {
    let source = r#"
```rust
let a = 100;
let b = "<>";
```
"#;
    let markdown_to_html = new_markdown_to_html_string(
        parser::Options::default(),
        html::Options {
            allows_unsafe: true,
            xhtml: false,
            ..html::Options::default()
        },
        parser::NO_EXTENSIONS,
        highlighting_html_renderer_extension(HighlightingHtmlRendererOptions {
            mode: HighlightingMode::Attribute,
            ..HighlightingHtmlRendererOptions::default()
        }),
    );
    MarkdownTestCase::new(
        1,
        "ok",
        source,
        r#"<pre style="background-color: #ffffff; padding: 12px; overflow: auto;"><code class="language-rust"><span style="font-weight:bold;color:#a71d5d;">let</span><span style="color:#323232;"> a </span><span style="font-weight:bold;color:#a71d5d;">= </span><span style="color:#0086b3;">100</span><span style="color:#323232;">;
</span><span style="font-weight:bold;color:#a71d5d;">let</span><span style="color:#323232;"> b </span><span style="font-weight:bold;color:#a71d5d;">= </span><span style="color:#183691;">&quot;&lt;&gt;&quot;</span><span style="color:#323232;">;
</span></code></pre>
"#,
        MarkdownTestCaseOptions::default(),
    )
    .execute(&markdown_to_html);
}

#[test]
fn test_highlighting_class() {
    let output = render("```rust\nlet a = 100;\n```\n", HighlightingMode::Class);
    assert!(
        output.starts_with(r#"<pre class="code"><code class="language-rust">"#),
        "{output}"
    );
    assert!(output.contains("<span class="), "{output}");
    assert!(output.ends_with("</code></pre>\n"), "{output}");
}

#[test]
fn test_unknown_language_falls_back() {
    assert_eq!(
        render("```nosuchlanguage\nfoo\n```\n", HighlightingMode::Attribute),
        "<pre><code class=\"language-nosuchlanguage\">foo\n</code></pre>\n"
    );
}

#[test]
fn test_no_language_falls_back() {
    assert_eq!(
        render("```\nfoo\n```\n", HighlightingMode::Attribute),
        "<pre><code>foo\n</code></pre>\n"
    );
}

#[test]
fn test_indented_code_block_falls_back() {
    assert_eq!(
        render("    foo\n", HighlightingMode::Attribute),
        "<pre><code>foo\n</code></pre>\n"
    );
}

#[test]
fn test_exclude_fn() {
    let options = HighlightingHtmlRendererOptions {
        exclude_fn: Some(Rc::new(|kd, source, _| {
            let language = kd.language(source);
            language.str(source) == "rust"
        })),
        ..HighlightingHtmlRendererOptions::default()
    };
    assert_eq!(
        render_with("```rust\nlet a = 100;\n```\n", options),
        "<pre><code class=\"language-rust\">let a = 100;\n</code></pre>\n"
    );
}

#[test]
fn test_exclude_languages() {
    let options = HighlightingHtmlRendererOptions {
        exclude_languages: &["rust", "text"],
        ..HighlightingHtmlRendererOptions::default()
    };
    assert_eq!(
        render_with("```rust\nlet a = 100;\n```\n", options),
        "<pre><code class=\"language-rust\">let a = 100;\n</code></pre>\n"
    );
}
