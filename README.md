# rushdown-highlighting
rushdown-highlighting is a server-side syntax highlighting plugin for [rushdown](https://github.com/yuin/rushdown), a markdown parser. 

## Installation
Add dependency to your `Cargo.toml`:

```toml
[dependencies]
rushdown-highlighting = "x.y.z"
```

## Usage
### Example

`````rust
use rushdown::{
    new_markdown_to_html,
    parser,
    renderer::html,
};
use rushdown_highlighting::{
    highlighting_html_renderer_extension, HighlightingHtmlRendererOptions, HighlightingMode,
};

let markdown_to_html = new_markdown_to_html(
    parser::Options::default(),
    html::Options::default(),
    parser::NO_EXTENSIONS,
    highlighting_html_renderer_extension(HighlightingHtmlRendererOptions {
        mode: HighlightingMode::Attribute,
        ..HighlightingHtmlRendererOptions::default()
    }),
);
let mut output = String::new();
let input = r#"
```rust
let a = 10;
```
"#;
match markdown_to_html(&mut output, input) {
    Ok(_) => {
        println!("HTML output:\n{}", output);
    }
    Err(e) => {
        println!("Error: {:?}", e);
    }
}
`````

Only fenced code blocks whose language is known by `syntect` are highlighted.
Other code blocks are rendered by the default renderer.

### Options

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `theme`| `&str` | `InspiredGitHub` | The name of the syntax highlighting theme to use. This option is only applicable when `mode` is set to `Attribute` |
| `mode` | `HighlightingMode` | `Attribute` | The mode to use for syntax highlighting. `Attribute` mode adds a `style` attribute to the HTML elements, while `Class` mode adds a `class` attribute. |
| `theme_set` | `Option<Rc<ThemeSet>>` | `None` | A custom set of syntax highlighting themes. If this option is not provided, the default themes from the `syntect` crate will be used. |
| `exclude_fn` | `Option<ExcludeFn>` | `None` | A function that decides whether a code block should be excluded from highlighting. Excluded code blocks are rendered by the default renderer. |
| `exclude_languages` | `&'static [&'static str]` | `&[]` | Languages to be excluded from highlighting. Excluded code blocks are rendered by the default renderer. |

```rust
use rushdown_highlighting::HighlightingHtmlRendererOptions;

let options = HighlightingHtmlRendererOptions {
    exclude_languages: &["text", "plaintext"],
    ..HighlightingHtmlRendererOptions::default()
};
```


## Donation
BTC: 1NEDSyUmo4SMTDP83JJQSWi1MvQUGGNMZB

Github sponsors also welcome.

## License
MIT

## Author
Yusuke Inuzuka
