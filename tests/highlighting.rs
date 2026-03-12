use rushdown::{
    new_markdown_to_html_string,
    parser::{self},
    renderer::html,
    test::{MarkdownTestCase, MarkdownTestCaseOptions},
};
use rushdown_highlighting::{
    highlighting_html_renderer_extension, HighlightingHtmlRendererOptions, HighlightingMode,
};

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
