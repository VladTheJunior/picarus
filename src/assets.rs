use anyhow::Result;
use anyhow::anyhow;
use gpui::App;
use gpui::RenderOnce;
use gpui::Window;
use gpui::{AssetSource, SharedString};
use gpui_component::Icon;
use gpui_component::IconNamed;
use gpui::IntoElement;
use gpui_component_macros::icon_named;
use rust_embed::Embed;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter().filter_map(|p| p.starts_with(path).then(|| p.into())).collect())
    }
}


icon_named!(AppIcon, "assets/icons");
impl RenderOnce for AppIcon {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}


#[derive(Embed)]
#[folder = "assets"]
#[include = "fonts/*"]
pub struct Fonts;
