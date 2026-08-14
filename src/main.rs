#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(unused_crate_dependencies)]
mod assets;

mod extensions;

mod game_data;
pub mod game_data_view;
mod language;
mod settings;

use gpui::{AppContext, Bounds, Global, ReadGlobal, Size, TitlebarOptions, WindowBounds, WindowOptions, px};
use gpui_component::{Root, Theme, ThemeConfig};

use std::rc::Rc;
use tracing::{Level, info};

use crate::{
    assets::{Assets, Fonts},
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

    tracing_subscriber::fmt().with_max_level(Level::DEBUG).init();

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

    let app = gpui_platform::application().with_assets(Assets);
    LanguageController::init();
    image_extras::register();

    app.run(move |cx| {
        let (settings, _) = Settings::try_load();
        cx.text_system()
            .add_fonts(Fonts::iter().map(|f| Fonts::get(&f)).flatten().map(|f| f.data).collect())
            .expect("Failed to load embedded font");
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        game_data_view::init(cx);
        Theme::global_mut(cx).apply_config(&dark_theme);
        Theme::global_mut(cx).scrollbar_mode = gpui_component::scroll::ScrollbarMode::Always;

        LanguageController::switch(settings.language);
        cx.set_global(settings);

        let bounds = Bounds::centered(None, Size::new(px(1340.0), px(700.0)), cx);
        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("PIcarus".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    window_min_size: Some(Size::new(px(800.0), px(400.0))),
                    ..Default::default()
                },
                |window, cx| {
                    cx.on_app_quit({
                        move |cx| {
                            Settings::global(cx).try_save();

                            async move {}
                        }
                    })
                    .detach();

                    let main_view = cx.new(|cx| GameDataView::new(window, cx));

                    cx.new(|cx| Root::new(main_view, window, cx))
                },
            )?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
