//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unused_crate_dependencies)]
mod extensions;

pub mod colors;
mod game_data;
pub mod game_data_view;
mod language;
pub mod rich_text;
mod settings;

use gpui_kit::assets::AllAssets;
use gpui_kit::component::{Root, Theme, ThemeConfig};
use gpui_kit::{AppContext, Bounds, Global, ReadGlobal, Size, TitlebarOptions, WindowBounds, WindowOptions, px};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::field::MakeExt;

use std::borrow::Cow;
use std::rc::Rc;
use tracing::info;

use crate::{
    game_data_view::GameDataView,
    language::LanguageController,
    settings::{Settings, config::Config},
};

impl Global for Settings {}

fn main() {
    // Initialize tokio runtime in the main thread
    // This is used for avoid tokio spawn hangs in the main thread.
    //
    // https://github.com/huacnlee/gpui-component/pull/100
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let _guard = rt.enter();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug,html5ever=off"));

    tracing_subscriber::fmt().map_fmt_fields(|f| f.debug_alt()).with_env_filter(filter).init();

    if let Some(timestamp) = option_env!("VERGEN_BUILD_TIMESTAMP") {
        info!("build timestamp: {timestamp}");
    }
    if let Some(semver) = option_env!("VERGEN_RUSTC_SEMVER") {
        info!("rustc: {semver}");
    }
    if let Some(branch) = option_env!("VERGEN_GIT_BRANCH") {
        info!("git branch: {branch}");
    }
    if let Some(describe) = option_env!("VERGEN_GIT_DESCRIBE") {
        info!("git describe: {describe}");
    }
    if let Some(timestamp) = option_env!("VERGEN_GIT_COMMIT_TIMESTAMP") {
        info!("git commit timestamp: {timestamp}");
    }
    let dark_theme =
        Rc::new(serde_json::from_slice::<ThemeConfig>(include_bytes!("../assets/themes/dark.json")).expect("Failed to parse dark theme"));

    LanguageController::init();
    image_extras::register();

    gpui_kit::application().with_assets(AllAssets).run(move |cx| {
        let (settings, _) = Settings::try_load();
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!("../assets/fonts/RobotoCondensed-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../assets/fonts/RobotoCondensed-Bold.ttf").as_slice()),
            ])
            .expect("Failed to load embedded font");

        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);
        game_data_view::init(cx);
        Theme::global_mut(cx).apply_config(&dark_theme);
        Theme::global_mut(cx).scrollbar_mode = gpui_kit::component::scroll::ScrollbarMode::Always;
        Theme::sync_base(cx);
        LanguageController::switch(settings.language);
        cx.set_global(settings);


        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, Size::new(px(1340.0), px(700.0)), cx))),
                titlebar: Some(TitlebarOptions {
                    title: Some("picarus".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_min_size: Some(Size::new(px(800.0), px(400.0))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.on_app_quit({
                    move |cx| {
                        Settings::global(cx).try_save();

                        async move {}
                    }
                })
                .detach();

                cx.new(|cx| GameDataView::new(window, cx))

                // cx.new(|_| HelloWorld)
            },
        )
        .expect("Failed to open window");
    });
}
