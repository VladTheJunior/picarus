use gpui_kit::{App, Hsla, IntoElement, RenderOnce, SharedString, Text, TextRun, TextStyle, accesskit::Color, div, hsla, px};
use html5ever::parse_document;

pub struct RichTextParser {
    pub segments: Vec<TextSegment>,
}

pub struct TextSegment {
    pub text: SharedString,
    pub color: Option<Hsla>,
}

impl RichTextParser {
    pub fn parse(html: &str) -> Self {
         let dom = parse_document(Sink::default(), Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())?;
        let document = Html::parse_fragment(html);
        let mut segments = Vec::new();

        // Process the root element's children
        for child in document.root_element().children() {
            Self::process_node(child, &mut spans, None);
        }

Self{segments}
    }
    fn get_tag_color(tag_name: Option<&str>)->Option<Hsla>{
        tag_name.and_then(|t| match t {
                    "UI_G" => Some(Hsla::green()),
                    "UI_R" => Some(Hsla::red()),
                   // "UI_Y" => Some(Hsla::yellow()),
                   // "UI_P" => Some(Hsla::purple()),

                    _ => None,
                })
    }

    fn process_node(node: NodeRef, spans: &mut Vec<Span>, tag_name: Option<&str>) {
        match node {
            Node::Text(text) => {
                if !text.is_empty() {
                    spans.push(Span {
                        text: SharedString::new(text.text),
                        color: None,
                    });
                }
            }
            Node::Element(element) => {
                for child in node.children() {
                    Self::process_node(child, spans, Some(element.name()));
                }
            }
            _ => {}
        }
    }

    fn apply_element_styles(&self, tag_name: &str, element: &scraper::Element, style: &mut TextStyle) {
        match tag_name {
            // Custom UI tags
            "UI_G" | "green" => style.color = Color::Green,
            "UI_R" | "red" => style.color = Color::Red,
            "UI_Y" | "yellow" => style.color = Color::Yellow,
            "UI_P" | "purple" => style.color = Color::Purple,

            // Handle custom hex color tags like UI_FF0000
            _ => {
                if let Some(color) = self.parse_hex_color_tag(tag_name) {
                    style.color = color;
                }
            }
        }
    }

    fn apply_class(&self, style: &mut TextStyle, class: &str) {
        for class_name in class.split_whitespace() {
            match class_name {
                "green" => style.color = Color::Green,
                "red" => style.color = Color::Red,

                "yellow" => style.color = Color::Yellow,
                "purple" => style.color = Color::Purple,

                _ => {}
            }
        }
    }

    fn apply_inline_style(&self, style: &mut TextStyle, style_attr: &str) {
        for part in style_attr.split(';') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim();

                match key.as_str() {
                    "color" => {
                        if let Some(color) = self.parse_color_value(value) {
                            style.color = color;
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}

// ============================================================================
// RichText Component
// ============================================================================

#[derive(Clone)]
pub struct RichText {
    runs: Vec<TextRun>,
}

impl RichText {
    pub fn new(html: &str) -> Self {
        let parser = RichTextParser::new(html);
        Self { runs: parser.parse() }
    }

    pub fn with_default_style(html: &str, default_style: TextStyle) -> Self {
        let parser = RichTextParser::new(html).with_default_style(default_style);
        Self { runs: parser.parse() }
    }

    pub fn from_html(html: &str) -> Self {
        Self::new(html)
    }

    // Helper to get the combined text without styles
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect::<Vec<&str>>().concat()
    }

    // Helper to check if text is empty
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty() || self.runs.iter().all(|run| run.text.is_empty())
    }
}

impl RenderOnce for RichText {
    fn render(self, window: &mut gpui_kit::Window, cx: &mut App) -> impl IntoElement {
        if self.is_empty() {
            return div().into_any_element();
        }

        // Build combined text with style refinements
        let combined_text: SharedString = self
            .runs
            .iter()
            .map(|run| run.text.clone())
            .collect::<Vec<SharedString>>()
            .concat()
            .into();

        let style_refinements = self.runs.into_iter().map(|run| run.style).collect::<Vec<TextStyle>>();

        Text::new("rich-text", combined_text)
            .with_text_style_override(Some(StyleRefinement {
                text: Some(style_refinements),
                ..Default::default()
            }))
            .into_any_element()
    }
}

// ============================================================================
// RichTextView - A View wrapper for use in GPUI views
// ============================================================================

pub struct RichTextView {
    pub content: RichText,
}

impl RichTextView {
    pub fn new(html: &str) -> Self {
        Self {
            content: RichText::new(html),
        }
    }

    pub fn with_style(html: &str, default_style: TextStyle) -> Self {
        Self {
            content: RichText::with_default_style(html, default_style),
        }
    }

    pub fn update(&mut self, html: &str) {
        self.content = RichText::new(html);
    }
}

impl RenderOnce for RichTextView {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        self.content.render(_cx)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Preprocess HTML to handle whitespace and normalize tags
pub fn preprocess_html(html: &str) -> String {
    let mut result = html.to_string();

    // Normalize custom tags to lowercase
    result = result.replace("<UI_G>", "<ui_g>");
    result = result.replace("</UI_G>", "</ui_g>");
    result = result.replace("<UI_R>", "<ui_r>");
    result = result.replace("</UI_R>", "</ui_r>");
    result = result.replace("<UI_B>", "<ui_b>");
    result = result.replace("</UI_B>", "</ui_b>");
    result = result.replace("<UI_Y>", "<ui_y>");
    result = result.replace("</UI_Y>", "</ui_y>");
    result = result.replace("<UI_P>", "<ui_p>");
    result = result.replace("</UI_P>", "</ui_p>");

    result
}
