use gpui::{
    Action, App, AppContext, ClickEvent, ClipboardItem, Context, Entity, FocusHandle, Focusable, Image, ImageSource, InteractiveElement, IntoElement,
    KeyBinding, ListSizingBehavior, ObjectFit, ParentElement, PathPromptOptions, ReadGlobal, Render, ScrollHandle, ScrollStrategy, SharedString,
    StatefulInteractiveElement, Styled, StyledImage, UniformListScrollHandle, UpdateGlobal, Window, actions, div, img, prelude::FluentBuilder, px,
    uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, InteractiveElementExt, Root, Sizable, StyledExt, TitleBar, WindowExt,
    button::{Button, ButtonVariants},
    combobox::Combobox,
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    notification::NotificationType,
    scroll::ScrollableElement,
    select::SearchableVec,
    spinner::Spinner,
    status_bar::StatusBar,
    switch::Switch,
    tab::{Tab, TabBar},
    v_flex,
};

use indexmap::{IndexMap, IndexSet};
use serde::Deserialize;

use std::{
    collections::{BTreeSet, HashMap},
    ops::Range,
    path::Path,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use tracing::error;

use crate::{
    assets::AppIcon,
    extensions::EnumNameExt,
    game_data::{
        DataType, GameClass, GameData, Grade, Item, ItemEffect, ItemMinMaxEffect, Quality,
        filters::{GameDataFilters, ItemEffectFilter},
        item_set::ItemSet,
    },
    language::{LanguageController, t, t_v},
    settings::Settings,
};

const CONTEXT: &str = "game_data";

actions!(game_data, [ClearSelection, CopySelection,]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Action)]
#[action(no_json)]
pub enum SelectionMove {
    Up,
    Down,
}

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", ClearSelection, Some(CONTEXT)),
        KeyBinding::new("ctrl-c", CopySelection, Some(CONTEXT)),
        KeyBinding::new("up", SelectionMove::Up, Some(CONTEXT)),
        KeyBinding::new("down", SelectionMove::Down, Some(CONTEXT)),
    ]);
}

#[derive(Debug)]
pub enum GameDataViewEvent {
    LoadingStep(SharedString),
    Reset,
}

#[derive(Default)]
struct PreviewValues {
    quality: Quality,
    total_tempering: u8,
    tempering: u8,
    reverse_tempering: u8,
    transcendence: u8,
    random_effects: HashMap<u8, ItemMinMaxEffect>,
}

impl PreviewValues {
    pub fn increase_transcendence(&mut self, transcendence_limit: u8) {
        self.transcendence = self.transcendence.saturating_add(1).min(transcendence_limit);
    }

    pub fn decrease_transcendence(&mut self) {
        self.transcendence = self.transcendence.saturating_sub(1);
    }

    pub fn increase_tempering(&mut self, temper_limit: u8, reverse_limit: u8) {
        if self.tempering < temper_limit {
            self.tempering = self.tempering.saturating_add(1).min(temper_limit);

            self.total_tempering = self.total_tempering.saturating_add(1).min(temper_limit + reverse_limit);
        }
    }

    pub fn increase_reverse_tempering(&mut self, temper_limit: u8, reverse_limit: u8) {
        if self.reverse_tempering < reverse_limit {
            self.reverse_tempering = self.reverse_tempering.saturating_add(1).min(reverse_limit);

            self.total_tempering = self.total_tempering.saturating_add(1).min(temper_limit + reverse_limit);
        }
    }

    pub fn decrease_tempering(&mut self) {
        if self.tempering > 0 {
            self.tempering = self.tempering.saturating_sub(1);
            self.total_tempering = self.total_tempering.saturating_sub(1);
        }
    }

    pub fn decrease_reverse_tempering(&mut self) {
        if self.reverse_tempering > 0 {
            self.reverse_tempering = self.reverse_tempering.saturating_sub(1);
            self.total_tempering = self.total_tempering.saturating_sub(1);
        }
    }
}

#[derive(Debug, Clone)]
pub enum GameDataLoadingStatus {
    ItemSet,
    SecondaryWeapon,
    Weapon,
    Accessory,
    Armor,
    Tempering,
    Effects,
    Quality,
}

impl GameDataLoadingStatus {
    pub fn localize(&self) -> SharedString {
        match self {
            GameDataLoadingStatus::ItemSet => t("game-data-loading-itemset"),
            GameDataLoadingStatus::Weapon => t("game-data-loading-weapon"),
            GameDataLoadingStatus::Accessory => t("game-data-loading-accessory"),
            GameDataLoadingStatus::Armor => t("game-data-loading-armor"),
            GameDataLoadingStatus::SecondaryWeapon => t("game-data-loading-secondary-weapon"),
            GameDataLoadingStatus::Tempering => t("game-data-loading-tempering"),
            GameDataLoadingStatus::Effects => t("game-data-loading-effects"),
            GameDataLoadingStatus::Quality => t("game-data-loading-quality"),
        }
    }
}

pub struct GameDataView {
    game_data: GameData,
    pub filters: GameDataFilters,
    pub filtered: IndexMap<SharedString, Rc<DataType>>,
    is_reading: bool,
    duration: usize,
    explorer_scroll_handle: UniformListScrollHandle,
    tabs_scroll_handle: ScrollHandle,
    focus_handle: FocusHandle,
    debug: bool,
    debug_preview: Entity<InputState>,
    loading_status: Entity<GameDataLoadingStatus>,
    game_path: Entity<InputState>,
    preview: HashMap<SharedString, PreviewValues>,
    tabs: IndexSet<SharedString>,
    selected_item: Option<SharedString>,
}

impl GameDataView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |view, cx| {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if view
                    .update(cx, |this, cx| {
                        if this.is_reading {
                            this.duration = this.duration.saturating_add(1);
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            game_data: GameData::new(),
            tabs: IndexSet::new(),
            filtered: IndexMap::new(),
            selected_item: None,
            filters: GameDataFilters::new(window, cx),
            is_reading: false,
            duration: 0,
            explorer_scroll_handle: UniformListScrollHandle::new(),
            tabs_scroll_handle: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            debug: false,
            preview: HashMap::new(),
            game_path: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Game path...")
                    .default_value(Settings::global(cx).game_path.clone())
            }),
            loading_status: cx.new(|_| GameDataLoadingStatus::Weapon),
            debug_preview: cx.new(|cx| InputState::new(window, cx).code_editor("json").multi_line(true)),
        }
    }

    pub fn apply_filter_and_resort(&mut self) {
        self.filtered = self
            .game_data
            .items
            .iter()
            .filter(|(_, item)| self.filters.check_item(item))
            .map(|(id, item)| (id.clone(), Rc::clone(item)))
            .collect();
        self.sort_by();
    }

    fn render_preview(
        id: SharedString,
        quality: Quality,
        temper_limit: u8,
        reverse_limit: u8,
        transcendence_limit: u8,
        transcendence_effect: Option<f32>,
        icon: Option<Arc<Image>>,
        grade: Option<Grade>,
        item_locale: SharedString,
        skill_locale: Option<SharedString>,
        usable_class: BTreeSet<GameClass>,
        attack: Option<(f32, f32, f32)>,
        attack_tempering_effect: Option<(f32, f32)>,
        physic_defense: Option<f32>,
        physic_defense_tempering_effect: Option<f32>,
        magic_defense: Option<f32>,
        magic_defense_tempering_effect: Option<f32>,
        attack_speed: Option<f32>,
        required_level: f32,
        min_sealed_slots: u8,
        max_sealed_slots: u8,
        min_random_effects: u8,
        max_random_effects: u8,
        random_effects: Option<Vec<ItemMinMaxEffect>>,
        item_set: Option<&ItemSet>,
        equip_effect_1: Option<ItemEffect>,
        equip_effect_2: Option<ItemEffect>,
        equip_effect_3: Option<ItemEffect>,
        equip_effect_4: Option<ItemEffect>,
        preview: &PreviewValues,
        items: &IndexMap<SharedString, Rc<DataType>>,
        decrease_transcendence_handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        increase_transcendence_handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        decrease_tempering_handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        increase_tempering_handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        decrease_reverse_tempering_handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        increase_reverse_tempering_handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        copy_item_name_handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        viewer_entity: Entity<GameDataView>,
        cx: &App,
    ) -> gpui::Div {
        v_flex()
            .items_start()
            .text_sm()
            .child(
                h_flex()
                    .gap_2()
                    .mb_2()
                    .items_start()
                    .child(
                        h_flex()
                            .when_none(&icon, |this| {
                                this.child(
                                    div()
                                        .size(px(128.))
                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color))
                                        .border_2(),
                                )
                            })
                            .when_some(icon, |this, icon| {
                                this.child(
                                    img(ImageSource::Image(icon))
                                        .object_fit(ObjectFit::Cover)
                                        .size(px(128.))
                                        .border_2()
                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color)),
                                )
                            }),
                    )
                    .child(
                        v_flex()
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .child(item_locale)
                                            .text_lg()
                                            .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color)),
                                    )
                                    .child(
                                        Button::new("copy-item-name")
                                            .icon(IconName::Copy)
                                            .ghost()
                                            .compact()
                                            .on_click(copy_item_name_handler),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(grade.map(|g| g.locale()).unwrap_or_else(|| t("item-unknown-grade")))
                                    .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                    .child(
                                        Button::new("button-quality")
                                            .small()
                                            .link()
                                            .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                            .label(quality.locale())
                                            .on_click({
                                                let viewer_entity: Entity<GameDataView> = viewer_entity.clone();
                                                let id = id.clone();
                                                move |_, _, cx| {
                                                    viewer_entity.update(cx, {
                                                        let id = id.clone();
                                                        move |this, cx| {
                                                            this.preview.entry(id.clone()).and_modify(|v| {
                                                                v.quality = v.quality.next();
                                                            });
                                                            cx.notify();
                                                        }
                                                    })
                                                }
                                            }),
                                    ),
                            )
                            .when_some(attack, |this, (dps, min, max)| {
                                this.child(h_flex().gap_1().child(format!("{} {:.1}", t("item-attack-dps"), dps)).when_some(
                                    attack_tempering_effect,
                                    |this, (dps_tempering_effect, _)| {
                                        this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", dps_tempering_effect)))
                                    },
                                ))
                                .child(
                                    h_flex().gap_1().child(format!("{} {} - {}", t("item-attack"), min, max)).when_some(
                                        attack_tempering_effect,
                                        |this, (_, min_max_tempering_effect)| {
                                            this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", min_max_tempering_effect)))
                                        },
                                    ),
                                )
                            })
                            .when_some(physic_defense, |this, physic_defense| {
                                this.child(
                                    h_flex()
                                        .gap_1()
                                        .child(format!("{} {:.1}", t("item-physical-defense"), physic_defense))
                                        .when_some(physic_defense_tempering_effect, |this, tempering_effect| {
                                            this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", tempering_effect)))
                                        }),
                                )
                            })
                            .when_some(magic_defense, |this, magic_defense| {
                                this.child(
                                    h_flex()
                                        .gap_1()
                                        .child(format!("{} {:.1}", t("item-magic-defense"), magic_defense))
                                        .when_some(magic_defense_tempering_effect, |this, tempering_effect| {
                                            this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", tempering_effect)))
                                        }),
                                )
                            })
                            .when_some(attack_speed, |this, attack_speed| {
                                this.child(format!("{} {:.1}", t("item-attack-speed"), attack_speed))
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_start()
                    .child(
                        v_flex()
                            .when_else(
                                transcendence_limit == 0,
                                |this| this.child(t("item-no-transcendence")),
                                |this| this.child(t("item-transcendence-limit")),
                            )
                            .when_else(
                                temper_limit == 0,
                                |this| this.child(t("item-no-tempering")),
                                |this| this.child(t("item-tempering-limit")),
                            )
                            .when_else(
                                reverse_limit == 0,
                                |this| this.child(t("item-no-reverse-tempering")),
                                |this| this.child(t("item-reverse-tempering-limit")),
                            ),
                    )
                    .child(
                        v_flex()
                            .justify_start()
                            .when_else(
                                transcendence_limit == 0,
                                |this| this.child(div().child(" ")),
                                |this| {
                                    this.child(
                                        h_flex()
                                            .child(
                                                Button::new("decrease-transcendence")
                                                    //   .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Minus)
                                                    .on_click(decrease_transcendence_handler),
                                            )
                                            .child(
                                                h_flex()
                                                    .justify_center()
                                                    .w(px(50.))
                                                    .child(format!("{}/{}", preview.transcendence, transcendence_limit)),
                                            )
                                            .child(
                                                Button::new("increase-transcendence")
                                                    //   .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Plus)
                                                    .on_click(increase_transcendence_handler),
                                            ),
                                    )
                                },
                            )
                            .when_else(
                                temper_limit == 0,
                                |this| this.child(div().child(" ")),
                                |this| {
                                    this.child(
                                        h_flex()
                                            .child(
                                                Button::new("decrease-tempering")
                                                    //   .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Minus)
                                                    .on_click(decrease_tempering_handler),
                                            )
                                            .child(
                                                h_flex()
                                                    .justify_center()
                                                    .w(px(50.))
                                                    .child(format!("{}/{}", preview.tempering, temper_limit)),
                                            )
                                            .child(
                                                Button::new("increase-tempering")
                                                    //   .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Plus)
                                                    .on_click(increase_tempering_handler),
                                            ),
                                    )
                                },
                            )
                            .when_else(
                                reverse_limit == 0,
                                |this| this.child(div().child(" ")),
                                |this| {
                                    this.child(
                                        h_flex()
                                            .child(
                                                Button::new("decrease-reverse-tempering")
                                                    //   .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Minus)
                                                    .on_click(decrease_reverse_tempering_handler),
                                            )
                                            .child(
                                                h_flex()
                                                    .justify_center()
                                                    .w(px(50.))
                                                    .child(format!("{}/{}", preview.reverse_tempering, reverse_limit)),
                                            )
                                            .child(
                                                Button::new("increase-reverse-tempering")
                                                    //   .ghost()
                                                    .xsmall()
                                                    .icon(IconName::Plus)
                                                    .on_click(increase_reverse_tempering_handler),
                                            ),
                                    )
                                },
                            ),
                    ),
            )
            .child(format!("{}: {}", t("item-required-level"), required_level))
            .when(max_sealed_slots > 0, |this| {
                this.child(format!("{}: {} - {}", t("item-sealed-stones-slots"), min_sealed_slots, max_sealed_slots))
            })
            .child(h_flex().gap_1().children(usable_class.iter().map(|c| c.locale())))
            .when_some(skill_locale, |this, skill_locale| {
                this.child(div().mt_2().text_color(cx.theme().success).child(t("item-equipped-skill")))
                    .child(div().text_color(cx.theme().yellow).child(skill_locale))
            })
            .when(max_random_effects > 0, {
                let viewer_entity = viewer_entity.clone();
                move |this| {
                    this.child(div().mt_2().text_color(cx.theme().yellow).child(format!(
                        "{} - {} {}",
                        min_random_effects,
                        max_random_effects,
                        t("item-random-equipped-effects")
                    )))
                    .when_some(random_effects, move |this, random_effects| {
                        this.children((0..max_random_effects).map(|i| {
                            let re = preview.random_effects.get(&i);

                            Button::new(format!("random-effects-dropdown-{}", i))
                                .small()
                                .text()
                                .cursor_pointer()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .when_some(re, |this, re| this.text_color(cx.theme().success).child(re.get_locale()))
                                        .when_none(&re, |this| {
                                            this.text_color(cx.theme().foreground).child(t("label-select-random-equipped-effect"))
                                        })
                                        .when_some(
                                            transcendence_effect.and_then(|transcendence_effect| {
                                                re.and_then(|f| f.parsed.as_ref())
                                                    .map(|(key, min, max)| (key, min * transcendence_effect, max * transcendence_effect))
                                            }),
                                            |this, (key, min_transcendence_effect, max_transcendence_effect)| {
                                                this.child(div().text_color(cx.theme().yellow).child(if key.ends_with("-minus-percent") {
                                                    format!("({:-.2}% ~ -{:.2}%)", min_transcendence_effect, max_transcendence_effect)
                                                } else if key.ends_with("-percent") {
                                                    format!("({:+.2}% ~ {:+.2}%)", min_transcendence_effect, max_transcendence_effect)
                                                } else {
                                                    format!("({:+.0} ~ {:+.0})", min_transcendence_effect, max_transcendence_effect)
                                                }))
                                            },
                                        ),
                                )
                                .dropdown_menu({
                                    let viewer_entity = viewer_entity.clone();
                                    let id = id.clone();
                                    {
                                        let random_effects = random_effects.clone();
                                        let id = id.clone();

                                        move |mut menu, window, _| {
                                            let id = id.clone();
                                            let random_effects = random_effects.clone();
                                            for e in random_effects.into_iter() {
                                                let id = id.clone();
                                                menu = menu.item(PopupMenuItem::new(e.get_locale()).on_click(window.listener_for(
                                                    &viewer_entity,
                                                    move |this, _, _, cx| {
                                                        this.preview.entry(id.clone()).and_modify(|v| {
                                                            v.random_effects.insert(i, e.clone());
                                                        });
                                                        cx.notify();
                                                    },
                                                )))
                                            }
                                            menu
                                        }
                                    }
                                })
                        }))
                    })
                }
            })
            .when_some(equip_effect_1, |this, equip_effect| {
                this.child(div().mt_2().text_color(cx.theme().yellow).child(t("item-equipped-effects")))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(div().text_color(cx.theme().success).child(equip_effect.get_locale()))
                            .when_some(
                                transcendence_effect.and_then(|transcendence_effect| {
                                    equip_effect.parsed.map(|(key, effect_value)| (key, effect_value * transcendence_effect))
                                }),
                                |this, (key, transcendence_effect)| {
                                    this.child(div().text_color(cx.theme().yellow).child(if key.ends_with("-minus-percent") {
                                        format!("({:-.2}%)", transcendence_effect)
                                    } else if key.ends_with("-percent") {
                                        format!("({:+.2}%)", transcendence_effect)
                                    } else {
                                        format!("({:+.0})", transcendence_effect)
                                    }))
                                },
                            ),
                    )
            })
            .when_some(equip_effect_2, |this, equip_effect| {
                this.child(
                    h_flex()
                        .gap_1()
                        .child(div().text_color(cx.theme().success).child(equip_effect.get_locale()))
                        .when_some(
                            transcendence_effect.and_then(|transcendence_effect| {
                                equip_effect.parsed.map(|(key, effect_value)| (key, effect_value * transcendence_effect))
                            }),
                            |this, (key, transcendence_effect)| {
                                this.child(div().text_color(cx.theme().yellow).child(if key.ends_with("-minus-percent") {
                                    format!("({:-.2}%)", transcendence_effect)
                                } else if key.ends_with("-percent") {
                                    format!("({:+.2}%)", transcendence_effect)
                                } else {
                                    format!("({:+.0})", transcendence_effect)
                                }))
                            },
                        ),
                )
            })
            .when_some(equip_effect_3, |this, equip_effect| {
                this.child(
                    h_flex()
                        .gap_1()
                        .child(div().text_color(cx.theme().success).child(equip_effect.get_locale()))
                        .when_some(
                            transcendence_effect.and_then(|transcendence_effect| {
                                equip_effect.parsed.map(|(key, effect_value)| (key, effect_value * transcendence_effect))
                            }),
                            |this, (key, transcendence_effect)| {
                                this.child(div().text_color(cx.theme().yellow).child(if key.ends_with("-minus-percent") {
                                    format!("({:-.2}%)", transcendence_effect)
                                } else if key.ends_with("-percent") {
                                    format!("({:+.2}%)", transcendence_effect)
                                } else {
                                    format!("({:+.0})", transcendence_effect)
                                }))
                            },
                        ),
                )
            })
            .when_some(equip_effect_4, |this, equip_effect| {
                this.child(
                    h_flex()
                        .gap_1()
                        .child(div().text_color(cx.theme().success).child(equip_effect.get_locale()))
                        .when_some(
                            transcendence_effect.and_then(|transcendence_effect| {
                                equip_effect.parsed.map(|(key, effect_value)| (key, effect_value * transcendence_effect))
                            }),
                            |this, (key, transcendence_effect)| {
                                this.child(div().text_color(cx.theme().yellow).child(if key.ends_with("-minus-percent") {
                                    format!("({:-.2}%)", transcendence_effect)
                                } else if key.ends_with("-percent") {
                                    format!("({:+.2}%)", transcendence_effect)
                                } else {
                                    format!("({:+.0})", transcendence_effect)
                                }))
                            },
                        ),
                )
            })
            .when_some(item_set, |this, item_set| {
                this.child(v_flex().child(div().mt_2().text_color(cx.theme().yellow).child(item_set.get_locale())))
                    .children(item_set.items.iter().map(|item| items.get(item)).map(|item| {
                        v_flex().items_start().when_some(item, {
                            let viewer_entity = viewer_entity.clone();
                            move |this, item| {
                                let id = item.get_id();
                                let grade = item.get_grade();
                                this.child(
                                    Button::new(format!("button-{}", id))
                                        .label(item.get_locale_name())
                                        .link()
                                        .small()
                                        .mb_2()
                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                        .on_click(move |_, window, cx| {
                                            cx.update_entity(&viewer_entity, |this, cx| {
                                                this.tabs.insert(id.clone());
                                                this.set_selected_item(Some(id.clone()), window, cx);
                                                cx.notify();
                                            })
                                        }),
                                )
                            }
                        })
                    }))
                    .when(!item_set.effects.is_empty(), |this| {
                        this.child(v_flex().gap_2().children(item_set.effects.iter().map(|set_effect| {
                            v_flex()
                                .text_sm()
                                .text_color(cx.theme().success)
                                .child(div().text_color(cx.theme().yellow).child(format!(
                                    "{} ({})",
                                    t("item-set-effects-count"),
                                    set_effect.seteffect_count
                                )))
                                .child(
                                    h_flex()
                                        .flex_wrap()
                                        .gap_x_2()
                                        .children(set_effect.seteffect_effects.iter().map(|effect| div().child(effect.get_locale()))),
                                )
                                .when_some(set_effect.get_locale(), |this, skill| this.child(skill))
                        })))
                    })
            })
        // .child(Self::render_item_set(items, item_set, cx))
    }

    fn sort_by(&mut self) {
        /*//let start = Instant::now();

        if self.sort_by_columns.is_empty() {
            self.filtered
                .par_sort_unstable_by(|_, a, _, b| a.debug.initial_position.cmp(&b.debug.initial_position));
            //debug!(elapsed=?start.elapsed(), count = self.filtered.len(), by_columns=?self.sort_by_columns, "Sort measure:");
            return;
        }

        self.filtered
            .par_sort_unstable_by(|_, a, _, b| Self::get_sort_ordering(a, b, &self.sort_by_columns));

        //debug!(elapsed=?start.elapsed(), count = self.filtered.len(), by_columns=?self.sort_by_columns, "Sort measure:");*/
    }
}

impl GameDataView {
    pub fn action_clear_selection(&mut self, _: &ClearSelection, window: &mut Window, cx: &mut Context<Self>) {
        self.set_selected_item(None, window, cx);
    }

    pub fn action_copy_selection(&mut self, _: &CopySelection, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_item) = self.selected_item.as_ref() else {
            return;
        };
        if let Some(item) = self.filtered.get(selected_item) {
            cx.write_to_clipboard(ClipboardItem::new_string(item.get_locale_name().to_string()));
            window.push_notification((NotificationType::Info, t("message-copy-item-name")), cx);
        }
    }

    pub fn set_selected_item(&mut self, item: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = item.as_ref() else {
            self.selected_item = None;
            cx.notify();
            return;
        };

        if let Some(item) = self.game_data.items.get(item) {
            self.selected_item = Some(item.get_id());
            self.debug_preview.update(cx, |state, cx| {
                state.set_value(serde_json::to_string_pretty(item.as_ref()).unwrap_or_default(), window, cx);
            });

            cx.notify();
        }
    }

    pub fn action_key_selection(&mut self, action: &SelectionMove, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_item) = self.selected_item.as_ref() else {
            return;
        };

        match action {
            SelectionMove::Up => {
                if let Some((ix, _, _)) = self.filtered.get_full(selected_item) {
                    if let Some((next_item, _)) = self.filtered.get_index(ix.saturating_sub(1)) {
                        self.set_selected_item(Some(next_item.clone()), window, cx);

                        self.explorer_scroll_handle.scroll_to_item(ix.saturating_sub(1), ScrollStrategy::Nearest);
                        cx.notify();
                    }
                }
            }
            SelectionMove::Down => {
                if let Some((ix, _, _)) = self.filtered.get_full(selected_item) {
                    if let Some((next_item, _)) = self.filtered.get_index(ix.saturating_add(1)) {
                        self.set_selected_item(Some(next_item.clone()), window, cx);
                        self.explorer_scroll_handle.scroll_to_item(ix.saturating_add(1), ScrollStrategy::Nearest);
                        cx.notify();
                    }
                }
            }
        }
    }

    fn close_tab(&mut self, item: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((index, _)) = self.tabs.shift_remove_full(item) {
            if self.tabs.is_empty() {
                self.set_selected_item(None, window, cx);
            } else if self.selected_item.as_ref().is_some_and(|f| f == item) {
                if index >= self.tabs.len() {
                    self.set_selected_item(self.tabs.last().map(|k| k.clone()), window, cx);
                } else {
                    self.set_selected_item(self.tabs.get_index(index).map(|k| k.clone()), window, cx);
                }

                cx.notify();
            }
        }
    }
}

impl Focusable for GameDataView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GameDataLoadingStatus {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(self.localize())
    }
}

impl Render for GameDataView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle(cx);
        let language = LanguageController::get_current_language();
        v_flex()
            .size_full()
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_1()
                        .child("PICARUS")
                        .when_some(option_env!("VERGEN_GIT_DESCRIBE"), |this, git_describe| {
                            this.child("-").child(
                                Button::new("button-github")
                                    .occlude()
                                    .text_color(cx.theme().foreground)
                                    .link()
                                    .small()
                                    .label(git_describe)
                                    .on_click(|_, _, cx| cx.open_url("https://github.com/VladTheJunior/picarus")),
                            )
                        }),
                ),
            )
            .when_else(
                self.is_reading,
                |this| {
                    this.child(
                        v_flex()
                            .size_full()
                            .justify_center()
                            .items_center()
                            .child(Spinner::new().icon(IconName::LoaderCircle).with_size(px(128.)))
                            .child(self.loading_status.clone())
                            .child(t_v("loading-duration-seconds", vec![("seconds", self.duration)])),
                    )
                },
                |this| {
                    this.when_else(
                        self.game_data.items.is_empty(),
                        |this| {
                            this.child(
                                v_flex()
                                    .size_full()
                                    .justify_center()
                                    .gap_3()
                                    .items_center()
                                    .child(Input::new(&self.game_path).disabled(true).w(px(300.)).suffix(
                                        Button::new("select-game-path").ghost().icon(IconName::FolderOpen).on_click(cx.listener(
                                            |_, _, window, cx| {
                                                let receiver = cx.prompt_for_paths(PathPromptOptions {
                                                    files: false,
                                                    directories: true,
                                                    multiple: false,
                                                    prompt: Some(t("button-select-game-folder")),
                                                });

                                                cx.spawn_in(window, async move |this, cx| {
                                                    if let Ok(Ok(Some(paths))) = receiver.await {
                                                        if let Some(selected_dir) = paths.first() {
                                                            let _ = this.update_in(cx, |this, window, cx| {
                                                                Settings::update_global(cx, |this, _| {
                                                                    this.game_path = selected_dir.to_string_lossy().to_string();
                                                                });
                                                                this.game_path.update(cx, |this, cx| {
                                                                    this.set_value(selected_dir.to_string_lossy().to_string(), window, cx)
                                                                });
                                                                cx.notify();
                                                            });
                                                        }
                                                    }
                                                })
                                                .detach();
                                            },
                                        )),
                                    ))
                                    .child(
                                        Button::new("read-game-data")
                                            .large()
                                            .label(t("button-load-game-data"))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.is_reading = true;
                                                let game_path = Settings::global(cx).game_path.clone();
                                                let loading_status = this.loading_status.clone();
                                                cx.spawn_in(window, async move |this, cx| {
                                                    let path = Path::new(&game_path);
                                                    if path.try_exists().ok().is_none_or(|f| f == false) || !path.is_dir() {
                                                        let _ = this.update_in(cx, |this, window, cx| {
                                                            window.push_notification(
                                                                (NotificationType::Error, t("message-game-folder-not-selected")),
                                                                cx,
                                                            );
                                                            this.is_reading = false;
                                                            cx.notify();
                                                        });
                                                        return;
                                                    }

                                                    if path.join(r"Game\gamedatas.npk").try_exists().ok().is_none_or(|f| f == false)
                                                        || path.join(r"Game\gamelibs.npk").try_exists().ok().is_none_or(|f| f == false)
                                                    {
                                                        let _ = this.update_in(cx, |this, window, cx| {
                                                            window.push_notification((NotificationType::Error, t("message-game-data-not-found")), cx);
                                                            this.is_reading = false;
                                                            cx.notify();
                                                        });
                                                        return;
                                                    }

                                                    match GameData::load(&game_path, &loading_status, cx).await {
                                                        Ok(data) => {
                                                            let _ = this.update_in(cx, |this, window, cx| {
                                                                this.game_data = data;
                                                                let effects = this.game_data.get_all_effects();

                                                                cx.update_entity(&this.filters.effects_state, |this, cx| {
                                                                    this.set_items(
                                                                        SearchableVec::new(
                                                                            effects
                                                                                .iter()
                                                                                .map(|key| ItemEffectFilter { key: key.clone() })
                                                                                .collect::<Vec<ItemEffectFilter>>(),
                                                                        ),
                                                                        window,
                                                                        cx,
                                                                    );
                                                                    this.set_selected_indices(vec![], window, cx);
                                                                    cx.notify();
                                                                });

                                                                this.apply_filter_and_resort();
                                                                this.is_reading = false;
                                                                cx.notify();
                                                            });
                                                        }
                                                        Err(e) => {
                                                            let _ = this.update_in(cx, |this, window, cx| {
                                                                window.push_notification(
                                                                    (NotificationType::Error, t("message-loading-data-failed")),
                                                                    cx,
                                                                );
                                                                this.is_reading = false;
                                                                cx.notify();
                                                            });
                                                            error!(?e, "Error while reading game data");
                                                        }
                                                    }
                                                })
                                                .detach();
                                            })),
                                    ),
                            )
                        },
                        |this| {
                            this.child(
                                h_flex()
                                    .size_full()
                                    .child(
                                        v_flex()
                                            .flex_grow_1()
                                            .size_full()
                                            .max_w(px(300.))
                                            .bg(cx.theme().table)
                                            .border_r_1()
                                            .border_color(cx.theme().border)
                                            .when_else(
                                                self.filtered.is_empty(),
                                                |this| {
                                                    this.child(
                                                        v_flex()
                                                            .size_full()
                                                            .items_center()
                                                            .justify_center()
                                                            .child(div().text_xl().font_bold().child(t("empty-list")))
                                                            .child(div().text_sm().text_center().child(t("empty-list-description"))),
                                                    )
                                                },
                                                |this| {
                                                    this.track_focus(&focus_handle)
                                                        .key_context(CONTEXT)
                                                        .on_action(window.listener_for(&cx.entity(), Self::action_clear_selection))
                                                        .on_action(window.listener_for(&cx.entity(), Self::action_key_selection))
                                                        .on_action(window.listener_for(&cx.entity(), Self::action_copy_selection))
                                                        .vertical_scrollbar(&self.explorer_scroll_handle)
                                                        .child(
                                                            uniform_list(
                                                                "data_items",
                                                                self.filtered.len(),
                                                                cx.processor(move |this, visible_range: Range<usize>, _, cx| {
                                                                    visible_range
                                                                        .filter_map(|ix| {
                                                                            this.filtered.iter().nth(ix).map(|(id, item)| {
                                                                                let icon = item.get_icon();
                                                                                let grade = item.get_grade();

                                                                                h_flex()
                                                                                    .w_full()
                                                                                    .id(id.clone())
                                                                                    .on_click(cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, window, cx| {
                                                                                            this.set_selected_item(Some(id.clone()), window, cx);
                                                                                            cx.notify();
                                                                                        }
                                                                                    }))
                                                                                    .on_double_click(cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, window, cx| {
                                                                                            this.tabs.insert(id.clone());
                                                                                            this.set_selected_item(Some(id.clone()), window, cx);
                                                                                            cx.notify();
                                                                                        }
                                                                                    }))
                                                                                    .py_1()
                                                                                    .px_3()
                                                                                    .gap_2()
                                                                                    .when(
                                                                                        this.selected_item.as_ref().is_some_and(|f| f == id),
                                                                                        |this| this.bg(cx.theme().selection),
                                                                                    )
                                                                                    .child(
                                                                                        h_flex()
                                                                                            .when_none(&icon, |this| {
                                                                                                this.child(
                                                                                                    div()
                                                                                                        .size(px(40.))
                                                                                                        .when_some(
                                                                                                            grade.and_then(|g| g.color()),
                                                                                                            |this, color| this.border_color(color),
                                                                                                        )
                                                                                                        .border_2(),
                                                                                                )
                                                                                            })
                                                                                            .when_some(icon, |this, icon| {
                                                                                                this.child(
                                                                                                    img(ImageSource::Image(icon))
                                                                                                        .object_fit(ObjectFit::Cover)
                                                                                                        .size(px(40.))
                                                                                                        .border_2()
                                                                                                        .when_some(
                                                                                                            grade.and_then(|g| g.color()),
                                                                                                            |this, color| this.border_color(color),
                                                                                                        ),
                                                                                                )
                                                                                            }),
                                                                                    )
                                                                                    .child(
                                                                                        div()
                                                                                            .truncate()
                                                                                            .w_full()
                                                                                            .text_sm()
                                                                                            .child(item.get_locale_name())
                                                                                            .when_some(
                                                                                                grade.and_then(|g| g.color()),
                                                                                                |this, color| this.text_color(color),
                                                                                            ),
                                                                                    )
                                                                            })
                                                                        })
                                                                        .collect()
                                                                }),
                                                            )
                                                            .flex_grow_1()
                                                            .size_full()
                                                            .track_scroll(&self.explorer_scroll_handle)
                                                            .with_sizing_behavior(ListSizingBehavior::Auto),
                                                        )
                                                },
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .min_w(px(300.))
                                            .size_full()
                                            .child(
                                                v_flex()
                                                    .id("outer-wrapper")
                                                    .flex_1()
                                                    // .min_h_0()
                                                    .horizontal_scrollbar(&self.tabs_scroll_handle)
                                                    .child(
                                                        TabBar::new("tabs")
                                                            .track_scroll(&self.tabs_scroll_handle)
                                                            .when_some(
                                                                self.selected_item.as_ref().and_then(|f| self.tabs.get_index_of(f)),
                                                                |this, index| this.selected_index(index),
                                                            )
                                                            .children(self.tabs.iter().map(|item_id| {
                                                                let item = self.game_data.items.get(item_id);

                                                                Tab::new()
                                                                    .child(div().px_1().w_full().when_some(item, |this, item| {
                                                                        let grade = item.get_grade();
                                                                        this.child(item.get_locale_name())
                                                                            .when_some(grade.and_then(|g| g.color()), |this, color| {
                                                                                this.text_color(color)
                                                                            })
                                                                    }))
                                                                    .on_click({
                                                                        let item_id = item_id.clone();
                                                                        cx.listener(move |this, _, window, cx| {
                                                                            this.set_selected_item(Some(item_id.clone()), window, cx);
                                                                            cx.notify();
                                                                        })
                                                                    })
                                                                    .suffix(
                                                                        Button::new(format!("close-{}", item_id))
                                                                            .icon(IconName::Close)
                                                                            .ghost()
                                                                            .xsmall()
                                                                            .on_click({
                                                                                let item_id = item_id.clone();
                                                                                cx.listener(move |view, _, window, cx| {
                                                                                    cx.stop_propagation();
                                                                                    view.close_tab(&item_id, window, cx);
                                                                                    cx.notify();
                                                                                })
                                                                            }),
                                                                    )
                                                            })),
                                                    ),
                                            )
                                            .when_some(
                                                self.selected_item
                                                    .as_ref()
                                                    .and_then(|selected_item_id| self.game_data.items.get(selected_item_id)),
                                                |this, selected_item| {
                                                    let icon = selected_item.get_icon();
                                                    let item_locale = selected_item.get_locale_name();

                                                    let grade = selected_item.get_grade();

                                                    let preview =
                                                        self.preview.entry(selected_item.get_id()).or_insert_with(|| PreviewValues::default());
                                                    let transcendence_effect =
                                                        (preview.transcendence != 0).then(|| preview.transcendence as f32 * 0.05);
                                                    let id = selected_item.get_id();
                                                    let (
                                                        quality,
                                                        temper_limit,
                                                        reverse_limit,
                                                        transcendence_limit,
                                                        skill_locale,
                                                        usable_class,
                                                        attack,
                                                        attack_tempering_effect,
                                                        physic_defense,
                                                        physic_defense_tempering_effect,
                                                        magic_defense,
                                                        magic_defense_tempering_effect,
                                                        attack_speed,
                                                        required_level,
                                                        min_sealed_slots,
                                                        max_sealed_slots,
                                                        min_random_effects,
                                                        max_random_effects,
                                                        random_effects,
                                                        item_set,
                                                        equip_effect_1,
                                                        equip_effect_2,
                                                        equip_effect_3,
                                                        equip_effect_4,
                                                    ) = match selected_item.as_ref() {
                                                        crate::game_data::DataType::SecondaryWeapon(secondary_weapon) => {
                                                            let transcendence_limit = secondary_weapon.overrise_max;
                                                            let temper_limit = secondary_weapon.enchant_limit;
                                                            let reverse_limit = secondary_weapon.reverse_enchant_limit;

                                                            let random_effects = grade
                                                                .and_then(|g| self.game_data.effects_by_grade.get(&g))
                                                                .and_then(|effects| effects.get(&secondary_weapon.item_level))
                                                                .and_then(|e| {
                                                                    GameClass::check_item_option(
                                                                        e,
                                                                        &secondary_weapon.usable_class,
                                                                        &secondary_weapon.weapon_type,
                                                                    )
                                                                });

                                                            let quality_effect = self
                                                                .game_data
                                                                .quality_by_types
                                                                .get(&secondary_weapon.get_type())
                                                                .and_then(|quality| quality.get(&secondary_weapon.item_level))
                                                                .and_then(|f| match preview.quality {
                                                                    Quality::Simple => None,
                                                                    Quality::Good => f
                                                                        .intermediate_fixed_effect
                                                                        .as_ref()
                                                                        .and_then(|f| f.parsed.as_ref())
                                                                        .map(|f| f.1),
                                                                    Quality::Perfect => {
                                                                        f.advanced_fixed_effect.as_ref().and_then(|f| f.parsed.as_ref()).map(|f| f.1)
                                                                    }
                                                                });

                                                            let (magic_tempering_effect, physical_tempering_effect) = self
                                                                .game_data
                                                                .tempering_by_types
                                                                .get(&secondary_weapon.get_full_type())
                                                                .and_then(|tempering| tempering.get(&secondary_weapon.item_level))
                                                                .and_then(|f| {
                                                                    preview.total_tempering.checked_sub(1).and_then(|index| {
                                                                        f.defense_ratios.get(index as usize).map(|f| {
                                                                            (
                                                                                f / 100.0 * secondary_weapon.magical_defense,
                                                                                f / 100.0
                                                                                    * (secondary_weapon.physical_defense
                                                                                        + quality_effect.unwrap_or_default()),
                                                                            )
                                                                        })
                                                                    })
                                                                })
                                                                .map_or((None, None), |(x, y)| (Some(x), Some(y)));

                                                            (
                                                                preview.quality,
                                                                temper_limit,
                                                                reverse_limit,
                                                                transcendence_limit,
                                                                secondary_weapon.get_skill_locale(),
                                                                secondary_weapon.usable_class.clone(),
                                                                None,
                                                                None,
                                                                Some(secondary_weapon.physical_defense + quality_effect.unwrap_or_default()),
                                                                physical_tempering_effect,
                                                                Some(secondary_weapon.magical_defense),
                                                                magic_tempering_effect,
                                                                None,
                                                                secondary_weapon.required_level,
                                                                0,
                                                                0,
                                                                secondary_weapon.random_option_count_min,
                                                                secondary_weapon.random_option_count_max,
                                                                random_effects,
                                                                secondary_weapon.item_set.as_ref(),
                                                                secondary_weapon.equip_effect_1.clone(),
                                                                secondary_weapon.equip_effect_2.clone(),
                                                                secondary_weapon.equip_effect_3.clone(),
                                                                secondary_weapon.equip_effect_4.clone(),
                                                            )
                                                        }

                                                        crate::game_data::DataType::Weapon(weapon) => {
                                                            let transcendence_limit = weapon.overrise_max;
                                                            let temper_limit = weapon.enhancement_limit;
                                                            let reverse_limit = weapon.reverse_enhancement_limit;
                                                            let random_effects = grade
                                                                .and_then(|g| self.game_data.effects_by_grade.get(&g))
                                                                .and_then(|effects| effects.get(&weapon.item_level))
                                                                .and_then(|e| {
                                                                    GameClass::check_item_option(e, &weapon.usable_class, &weapon.weapon_type)
                                                                });

                                                            let quality_effect = self
                                                                .game_data
                                                                .quality_by_types
                                                                .get(&weapon.get_type())
                                                                .and_then(|quality| quality.get(&weapon.item_level))
                                                                .and_then(|f| match preview.quality {
                                                                    Quality::Simple => None,
                                                                    Quality::Good => f
                                                                        .intermediate_fixed_effect
                                                                        .as_ref()
                                                                        .and_then(|f| f.parsed.as_ref())
                                                                        .map(|f| f.1),
                                                                    Quality::Perfect => {
                                                                        f.advanced_fixed_effect.as_ref().and_then(|f| f.parsed.as_ref()).map(|f| f.1)
                                                                    }
                                                                });

                                                            let tempering_effect = self
                                                                .game_data
                                                                .tempering_by_types
                                                                .get(&weapon.get_full_type())
                                                                .and_then(|tempering| tempering.get(&weapon.item_level))
                                                                .and_then(|f| {
                                                                    if weapon.attack_range_type == "me" {
                                                                        preview.total_tempering.checked_sub(1).and_then(|index| {
                                                                            f.defenses.get(index as usize).map(|f| {
                                                                                (
                                                                                    f / 100.0
                                                                                        * (weapon.max_attack
                                                                                            + quality_effect.unwrap_or_default()
                                                                                            + weapon.min_attack
                                                                                            + quality_effect.unwrap_or_default())
                                                                                        / 2.0
                                                                                        / weapon.attack_speed
                                                                                        + quality_effect.unwrap_or_default(),
                                                                                    f / 100.0
                                                                                        * (weapon.max_attack
                                                                                            + quality_effect.unwrap_or_default()
                                                                                            + weapon.min_attack
                                                                                            + quality_effect.unwrap_or_default())
                                                                                        / 2.0,
                                                                                )
                                                                            })
                                                                        })
                                                                    } else {
                                                                        preview.total_tempering.checked_sub(1).and_then(|index| {
                                                                            f.spell_ratios.get(index as usize).map(|f| {
                                                                                (
                                                                                    f / 100.0
                                                                                        * (weapon.max_attack
                                                                                            + quality_effect.unwrap_or_default()
                                                                                            + weapon.min_attack
                                                                                            + quality_effect.unwrap_or_default())
                                                                                        / 2.0
                                                                                        / weapon.attack_speed,
                                                                                    f / 100.0
                                                                                        * (weapon.max_attack
                                                                                            + quality_effect.unwrap_or_default()
                                                                                            + weapon.min_attack
                                                                                            + quality_effect.unwrap_or_default())
                                                                                        / 2.0,
                                                                                )
                                                                            })
                                                                        })
                                                                    }
                                                                });

                                                            (
                                                                preview.quality,
                                                                temper_limit,
                                                                reverse_limit,
                                                                transcendence_limit,
                                                                weapon.get_skill_locale(),
                                                                weapon.usable_class.clone(),
                                                                Some((
                                                                    (weapon.max_attack
                                                                        + quality_effect.unwrap_or_default()
                                                                        + weapon.min_attack
                                                                        + quality_effect.unwrap_or_default())
                                                                        / 2.0
                                                                        / weapon.attack_speed,
                                                                    weapon.min_attack + quality_effect.unwrap_or_default(),
                                                                    weapon.max_attack + quality_effect.unwrap_or_default(),
                                                                )),
                                                                tempering_effect,
                                                                None,
                                                                None,
                                                                None,
                                                                None,
                                                                Some(weapon.attack_speed),
                                                                weapon.required_level,
                                                                weapon.min_crafting_seal_slots,
                                                                weapon.max_crafting_seal_slots,
                                                                weapon.min_random_options,
                                                                weapon.max_random_options,
                                                                random_effects,
                                                                weapon.item_set.as_ref(),
                                                                weapon.equip_effect_1.clone(),
                                                                weapon.equip_effect_2.clone(),
                                                                weapon.equip_effect_3.clone(),
                                                                weapon.equip_effect_4.clone(),
                                                            )
                                                        }

                                                        crate::game_data::DataType::Armor(armor) => {
                                                            let temper_limit = armor.enhancement_limit;
                                                            let reverse_limit = armor.reverse_enhancement_limit;
                                                            let transcendence_limit = armor.overrise_max;
                                                            let random_effects = grade
                                                                .and_then(|g| self.game_data.effects_by_grade.get(&g))
                                                                .and_then(|effects| effects.get(&armor.item_level))
                                                                .and_then(|e| {
                                                                    GameClass::check_item_option(
                                                                        e,
                                                                        &armor.usable_class,
                                                                        &format!("{}_{}", armor.armor_type, armor.equip_slot),
                                                                    )
                                                                });

                                                            let quality_effect = self
                                                                .game_data
                                                                .quality_by_types
                                                                .get(&armor.get_type())
                                                                .and_then(|quality| quality.get(&armor.item_level))
                                                                .and_then(|f| match preview.quality {
                                                                    Quality::Simple => None,
                                                                    Quality::Good => f
                                                                        .intermediate_fixed_effect
                                                                        .as_ref()
                                                                        .and_then(|f| f.parsed.as_ref())
                                                                        .map(|f| f.1),
                                                                    Quality::Perfect => {
                                                                        f.advanced_fixed_effect.as_ref().and_then(|f| f.parsed.as_ref()).map(|f| f.1)
                                                                    }
                                                                });

                                                            let (magic_tempering_effect, physical_tempering_effect) = self
                                                                .game_data
                                                                .tempering_by_types
                                                                .get(&armor.get_full_type())
                                                                .and_then(|tempering| tempering.get(&armor.item_level))
                                                                .and_then(|f| {
                                                                    preview.total_tempering.checked_sub(1).and_then(|index| {
                                                                        f.defense_ratios.get(index as usize).map(|f| {
                                                                            (
                                                                                f / 100.0
                                                                                    * (armor.magical_defense + quality_effect.unwrap_or_default()),
                                                                                f / 100.0
                                                                                    * (armor.physical_defense + quality_effect.unwrap_or_default()),
                                                                            )
                                                                        })
                                                                    })
                                                                })
                                                                .map_or((None, None), |(x, y)| (Some(x), Some(y)));

                                                            (
                                                                preview.quality,
                                                                temper_limit,
                                                                reverse_limit,
                                                                transcendence_limit,
                                                                armor.get_skill_locale(),
                                                                armor.usable_class.clone(),
                                                                None,
                                                                None,
                                                                Some(armor.physical_defense + quality_effect.unwrap_or_default()),
                                                                physical_tempering_effect,
                                                                Some(armor.magical_defense + quality_effect.unwrap_or_default()),
                                                                magic_tempering_effect,
                                                                None,
                                                                armor.required_level,
                                                                armor.sealed_fellow_slots_min,
                                                                armor.sealed_fellow_slots_max,
                                                                armor.random_option_count_min,
                                                                armor.random_option_count_max,
                                                                random_effects,
                                                                armor.item_set.as_ref(),
                                                                armor.equip_effect_1.clone(),
                                                                armor.equip_effect_2.clone(),
                                                                armor.equip_effect_3.clone(),
                                                                armor.equip_effect_4.clone(),
                                                            )
                                                        }

                                                        crate::game_data::DataType::Accessory(accessory) => {
                                                            let temper_limit = accessory.enhancement_limit;
                                                            let reverse_limit = accessory.reverse_enhancement_limit;
                                                            let transcendence_limit = accessory.overrise_max;
                                                            let random_effects = grade
                                                                .and_then(|g| self.game_data.effects_by_grade.get(&g))
                                                                .and_then(|effects| effects.get(&accessory.item_level))
                                                                .and_then(|e| {
                                                                    GameClass::check_item_option(
                                                                        e,
                                                                        &accessory.usable_class,
                                                                        &accessory.accessory_type,
                                                                    )
                                                                });

                                                            let quality_effect = self
                                                                .game_data
                                                                .quality_by_types
                                                                .get(&accessory.get_type())
                                                                .and_then(|quality| quality.get(&accessory.item_level))
                                                                .and_then(|f| match preview.quality {
                                                                    Quality::Simple => None,
                                                                    Quality::Good => f
                                                                        .intermediate_fixed_effect
                                                                        .as_ref()
                                                                        .and_then(|f| f.parsed.as_ref())
                                                                        .map(|f| f.1),
                                                                    Quality::Perfect => {
                                                                        f.advanced_fixed_effect.as_ref().and_then(|f| f.parsed.as_ref()).map(|f| f.1)
                                                                    }
                                                                });

                                                            let tempering_effect = self
                                                                .game_data
                                                                .tempering_by_types
                                                                .get(&accessory.get_full_type())
                                                                .and_then(|tempering| tempering.get(&accessory.item_level))
                                                                .and_then(|f| {
                                                                    preview.total_tempering.checked_sub(1).and_then(|index| {
                                                                        f.defense_ratios.get(index as usize).map(|f| {
                                                                            f / 100.0 * (accessory.magic_defense + quality_effect.unwrap_or_default())
                                                                        })
                                                                    })
                                                                });

                                                            (
                                                                preview.quality,
                                                                temper_limit,
                                                                reverse_limit,
                                                                transcendence_limit,
                                                                accessory.get_skill_locale(),
                                                                accessory.usable_class.clone(),
                                                                None,
                                                                None,
                                                                None,
                                                                None,
                                                                Some(accessory.magic_defense + quality_effect.unwrap_or_default()),
                                                                tempering_effect,
                                                                None,
                                                                accessory.required_level,
                                                                0,
                                                                0,
                                                                accessory.random_option_count_min,
                                                                accessory.random_option_count_max,
                                                                random_effects,
                                                                accessory.item_set.as_ref(),
                                                                accessory.equip_effect_1.clone(),
                                                                accessory.equip_effect_2.clone(),
                                                                accessory.equip_effect_3.clone(),
                                                                accessory.equip_effect_4.clone(),
                                                            )
                                                        }
                                                    };
                                                    /* */
                                                    this.child(
                                                        div()
                                                            .flex()
                                                            .size_full()
                                                            .relative()
                                                            .when_else(
                                                                self.debug,
                                                                |this| {
                                                                    this.child(
                                                                        Input::new(&self.debug_preview)
                                                                            .font_family(cx.theme().mono_font_family.clone())
                                                                            .text_size(cx.theme().mono_font_size)
                                                                            .disabled(true)
                                                                            .border_0()
                                                                            .rounded_none()
                                                                            .size_full(),
                                                                    )
                                                                },
                                                                |this| {
                                                                    this.child(
                                                                        v_flex()
                                                                            .min_h_0()
                                                                            .id("item-preview")
                                                                            .p_2()
                                                                            .size_full()
                                                                            .overflow_y_scrollbar()
                                                                            .map(|this| {
                                                                                this.child(Self::render_preview(
                                                                                    id.clone(),
                                                                                    quality,
                                                                                    temper_limit,
                                                                                    reverse_limit,
                                                                                    transcendence_limit,
                                                                                    transcendence_effect,
                                                                                    icon,
                                                                                    grade,
                                                                                    item_locale.clone(),
                                                                                    skill_locale,
                                                                                    usable_class,
                                                                                    attack,
                                                                                    attack_tempering_effect,
                                                                                    physic_defense,
                                                                                    physic_defense_tempering_effect,
                                                                                    magic_defense,
                                                                                    magic_defense_tempering_effect,
                                                                                    attack_speed,
                                                                                    required_level,
                                                                                    min_sealed_slots,
                                                                                    max_sealed_slots,
                                                                                    min_random_effects,
                                                                                    max_random_effects,
                                                                                    random_effects,
                                                                                    item_set,
                                                                                    equip_effect_1,
                                                                                    equip_effect_2,
                                                                                    equip_effect_3,
                                                                                    equip_effect_4,
                                                                                    preview,
                                                                                    &self.game_data.items,
                                                                                    cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, _, cx| {
                                                                                            let preview = this
                                                                                                .preview
                                                                                                .entry(id.clone())
                                                                                                .or_insert_with(|| PreviewValues::default());
                                                                                            preview.decrease_transcendence();
                                                                                            cx.notify();
                                                                                        }
                                                                                    }),
                                                                                    cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, _, cx| {
                                                                                            let preview = this
                                                                                                .preview
                                                                                                .entry(id.clone())
                                                                                                .or_insert_with(|| PreviewValues::default());
                                                                                            preview.increase_transcendence(transcendence_limit);

                                                                                            cx.notify();
                                                                                        }
                                                                                    }),
                                                                                    cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, _, cx| {
                                                                                            let preview = this
                                                                                                .preview
                                                                                                .entry(id.clone())
                                                                                                .or_insert_with(|| PreviewValues::default());
                                                                                            preview.decrease_tempering();
                                                                                            cx.notify();
                                                                                        }
                                                                                    }),
                                                                                    cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, _, cx| {
                                                                                            let preview = this
                                                                                                .preview
                                                                                                .entry(id.clone())
                                                                                                .or_insert_with(|| PreviewValues::default());
                                                                                            preview.increase_tempering(temper_limit, reverse_limit);

                                                                                            cx.notify();
                                                                                        }
                                                                                    }),
                                                                                    cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, _, cx| {
                                                                                            let preview = this
                                                                                                .preview
                                                                                                .entry(id.clone())
                                                                                                .or_insert_with(|| PreviewValues::default());
                                                                                            preview.decrease_reverse_tempering();
                                                                                            cx.notify();
                                                                                        }
                                                                                    }),
                                                                                    cx.listener({
                                                                                        let id = id.clone();
                                                                                        move |this, _, _, cx| {
                                                                                            let preview = this
                                                                                                .preview
                                                                                                .entry(id.clone())
                                                                                                .or_insert_with(|| PreviewValues::default());
                                                                                            preview.increase_reverse_tempering(
                                                                                                temper_limit,
                                                                                                reverse_limit,
                                                                                            );

                                                                                            cx.notify();
                                                                                        }
                                                                                    }),
                                                                                    cx.listener({
                                                                                        let item_locale = item_locale.clone();
                                                                                        move |_, _, window, cx| {
                                                                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                                                                item_locale.to_string(),
                                                                                            ));
                                                                                            window.push_notification(
                                                                                                (NotificationType::Info, t("message-copy-item-name")),
                                                                                                cx,
                                                                                            );
                                                                                        }
                                                                                    }),
                                                                                    cx.entity(),
                                                                                    cx,
                                                                                ))
                                                                            }),
                                                                    )
                                                                },
                                                            )
                                                            .child(
                                                                Switch::new("debug-switch")
                                                                    .absolute()
                                                                    .bottom(px(16.0))
                                                                    .right(px(16.0))
                                                                    .checked(self.debug)
                                                                    .tooltip(t("tooltip-debug-switch"))
                                                                    .on_click(cx.listener(|this, checked, _, cx| {
                                                                        this.debug = *checked;
                                                                        cx.notify();
                                                                    })),
                                                            ),
                                                    )
                                                },
                                            )
                                            .into_any_element(),
                                    ),
                            )
                        },
                    )
                },
            )
            .child(
                StatusBar::new()
                    .left(
                        Input::new(&self.filters.search_state)
                            .flex_shrink_0()
                            .w(px(290.))
                            .disabled(self.is_reading)
                            .prefix(Icon::new(IconName::Search).small())
                            .small(),
                    )
                    .left(
                        Combobox::new(&self.filters.item_type_state)
                            .w(px(150.))
                            .flex_shrink_0()
                            .disabled(self.is_reading)
                            .small()
                            .render_trigger(move |_, _, _| div().child(t("item-types"))),
                    )
                    .left(
                        Combobox::new(&self.filters.grade_state)
                            .w(px(150.))
                            .flex_shrink_0()
                            .disabled(self.is_reading)
                            .small()
                            .render_trigger(move |_, _, _| div().child(t("item-grades"))),
                    )
                    .left(
                        Combobox::new(&self.filters.effects_state)
                            .w(px(300.))
                            .flex_shrink_0()
                            .disabled(self.is_reading)
                            .small()
                            .cleanable(true)
                            .placeholder(t("item-effects")),
                    )
                    .right(
                        Button::new("lang-switcher")
                            .icon(AppIcon::Languages)
                            .ghost()
                            .xsmall()
                            .w(px(70.))
                            .cursor_pointer()
                            .label(language.title())
                            .on_click(cx.listener(move |_, _, _, cx| {
                                match language {
                                    crate::settings::Language::English => {
                                        Settings::update_global(cx, |this, _| this.language = crate::settings::Language::Russian);
                                        LanguageController::switch(crate::settings::Language::Russian);
                                    }
                                    crate::settings::Language::Russian => {
                                        Settings::update_global(cx, |this, _| this.language = crate::settings::Language::English);
                                        LanguageController::switch(crate::settings::Language::English);
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            )
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}
