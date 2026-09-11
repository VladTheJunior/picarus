use std::sync::LazyLock;

use gpui_kit::{App, Font, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, StyledText, TextRun, div};
use regex::Regex;
use scraper::{ElementRef, Html, Node};
use tracing::warn;

use crate::colors::{GREEN, ORANGE, PURPLE, RED};

#[derive(IntoElement)]
pub struct RichText {
    pub text: SharedString,
    pub segments: Vec<TextRun>,
}

static TAGS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n+").unwrap());

fn remove_html_multilines_regex(text: &str) -> String {
    TAGS_RE.replace_all(&text.replace("<br>", "\n").replace("<br />", "\n"), "\n").to_string()
}

pub struct TextSegment {
    pub text: SharedString,
    pub color: Option<Hsla>,
}

impl RichText {
    pub fn parse(html: &str, default_text_color: Hsla) -> Self {
        let html = remove_html_multilines_regex(html);
        if !html.contains('<') {
            return Self {
                text: SharedString::new(html.clone()),
                segments: vec![TextRun {
                    len: html.len(),
                    color: default_text_color,
                    font: Font {
                        family: "Roboto Condensed".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
            };
        }

        let document = Html::parse_fragment(&html);
        let mut segments = Vec::new();
        let mut current_text = String::new();
        let mut current_color = None;
        let mut color_stack = Vec::new();
        let mut text = String::new();
        Self::process_node(
            document.root_element(),
            &mut text,
            &mut segments,
            &mut current_text,
            &mut current_color,
            &mut color_stack,
            default_text_color,
        );

        Self {
            text: SharedString::new(text),
            segments,
        }
    }
    fn get_color_by_tag(tag_name: &str) -> Option<Hsla> {
        match tag_name.to_uppercase().as_str() {
            "UI_G" | "G_Y" | "IT_G" => Some(GREEN),
            "UI_R" | "R_Y" => Some(RED),
            "O_Y" => Some(ORANGE),
            "UI_P" => Some(PURPLE),
            "BR" | "UI_Y" => None,
            _ => {
                warn!(?tag_name, "unknown tag");
                None
            }
        }
    }

    fn process_node(
        element: ElementRef,
        result: &mut String,
        segments: &mut Vec<TextRun>,
        current_text: &mut String,
        current_color: &mut Option<Hsla>,
        color_stack: &mut Vec<Option<Hsla>>,
        default_text_color: Hsla,
    ) {
        for node in element.children() {
            match node.value() {
                Node::Text(text) => {
                    if !text.is_empty() {
                        current_text.push_str(&text.text);
                    }
                }
                Node::Element(element) => {
                    if !current_text.is_empty() {
                        segments.push(TextRun {
                            len: current_text.len(),

                            color: current_color.unwrap_or_else(|| default_text_color),
                            font: Font {
                                family: "Roboto Condensed".into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        result.push_str(&current_text);
                        current_text.clear();
                    }

                    let new_color = Self::get_color_by_tag(element.name());
                    color_stack.push(current_color.clone());
                    *current_color = new_color;

                    Self::process_node(
                        ElementRef::wrap(node).expect("Failed to wrap element"),
                        result,
                        segments,
                        current_text,
                        current_color,
                        color_stack,
                        default_text_color,
                    );

                    if let Some(previous_color) = color_stack.pop() {
                        // Flush any remaining text before restoring previous color
                        if !current_text.is_empty() {
                            segments.push(TextRun {
                                len: current_text.len(),

                                color: current_color.unwrap_or_else(|| default_text_color),
                                font: Font {
                                    family: "Roboto Condensed".into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                            result.push_str(&current_text);
                            current_text.clear();
                        }
                        *current_color = previous_color;
                    }
                }
                _ => {}
            }
        }
    }
}

impl RenderOnce for RichText {
    fn render(self, _window: &mut gpui_kit::Window, _cx: &mut App) -> impl IntoElement {
        return div().child(StyledText::new(self.text).with_runs(self.segments));
    }
}
