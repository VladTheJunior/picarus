use gpui_kit::Div;
use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Root, Sizable, StyledExt, TitleBar, WindowExt,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    combobox::{Combobox, ComboboxEvent},
    h_flex,
    input::{Editor, EditorState, Input, InputState},
    label::Label,
    menu::{DropdownMenu, PopupMenuItem},
    notification::{Notification, NotificationType},
    progress::ProgressCircle,
    scroll::ScrollableElement,
    select::SearchableVec,
    separator::Separator,
    status_bar::StatusBar,
    switch::Switch,
    tab::{Tab, TabBar},
    tooltip::Tooltip,
    v_flex,
};
use gpui_kit::{
    Action, App, AppContext, ClipboardItem, Context, Entity, FocusHandle, Focusable, FontWeight, ImageSource, InteractiveElement, IntoElement,
    KeyBinding, ListSizingBehavior, ObjectFit, ParentElement, PathPromptOptions, ReadGlobal, Render, ScrollHandle, ScrollStrategy, SharedString,
    StatefulInteractiveElement, Styled, StyledImage, UniformListScrollHandle, UpdateGlobal, Window, actions, div, img, prelude::FluentBuilder, px,
    rems, rgb, uniform_list,
};

use indexmap::{IndexMap, IndexSet};

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::Deserialize;
use strum::IntoEnumIterator;

use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    ops::Range,
    path::Path,
    rc::{Rc, Weak},
};

use crate::game_data::filters::AdditionalFilter;
use crate::game_data::items::fellow::FellowType;
use crate::rich_text::RichText;
use crate::{
    assets::AppIcon,
    extensions::EnumNameExt,
    game_data::{
        GameData,
        common::Common,
        effects::{ItemMinMaxEffect, ItemMinMaxNoStepEffect, ItemMinMaxStepEffect},
        filters::{GameDataFilters, ItemEffectFilter},
        grade::Grade,
        items::{Item, ItemTrait, ItemType, package::PackageItem, recipe::RecipeType},
        product::Product,
        quality::Quality,
        random_box_group::RandomBoxGroup,
        skill::Skill,
    },
    language::{LanguageController, t, t_v},
    settings::Settings,
};
use anyhow::Result;
use tracing::{error, warn};

const CONTEXT: &str = "game_data";

actions!(game_data, [ClearSelection, CopySelection,]);

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Action)]
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

pub enum GameDataViewEvent {
    LoadingStep(SharedString),
    Reset,
}

pub struct PreviewBuilder<'a> {
    pub common: &'a Common,
    pub optional: PreviewBuilderOptional,
}

#[derive(Default)]
pub struct PreviewBuilderOptional {
    pub quality_effect: Option<f32>,
    pub temper_limit: Option<u8>,
    pub reverse_limit: Option<u8>,
    pub transcendence_limit: Option<u8>,
    pub transcendence_effect: Option<f32>,
    pub skill_locale: Option<SharedString>,
    pub description_locale: Option<SharedString>,
    pub max_gem_slots: Option<u8>,
    pub attack: Option<(f32, f32, f32)>,
    pub attack_tempering_effect: Option<(f32, f32)>,
    pub physic_defense: Option<f32>,
    pub physic_defense_tempering_effect: Option<f32>,
    pub magic_defense: Option<f32>,
    pub magic_defense_tempering_effect: Option<f32>,
    pub talent_power: Option<u16>,

    pub recipe_types: Option<BTreeSet<RecipeType>>,
    pub recipe_stage: Option<u8>,
    pub product: Option<Weak<RefCell<Product>>>,

    pub min_sealed_slots: u8,
    pub max_sealed_slots: u8,
    pub min_random_effects: u8,
    pub max_random_effects: u8,
    pub random_effects: Option<Vec<ItemMinMaxEffect>>,

    pub fellow_stone_effects: Option<(Vec<ItemMinMaxStepEffect>, Option<ItemMinMaxNoStepEffect>, u8, f32)>,

    pub max_ep: Option<f32>,

    pub random_box_group: Option<Weak<RefCell<RandomBoxGroup>>>,
    pub package_contents: Option<IndexMap<usize, PackageItem>>,

    pub skills: Vec<Skill>,
    pub fellow_speed: Option<f32>,
    pub fellow_adventure_points: Option<u16>,
    pub fellow_type: Option<FellowType>,
    pub fellow_can_fly: Option<bool>,
    pub fellow_people: Option<u8>,
    pub fellow_region: Option<SharedString>,
}

impl<'a> PreviewBuilder<'a> {
    pub fn new(common: &'a Common) -> Self {
        Self {
            common,
            optional: PreviewBuilderOptional::default(),
        }
    }
    pub fn fellow_type(mut self, value: FellowType) -> Self {
        self.optional.fellow_type = Some(value);
        self
    }

    pub fn fellow_can_fly(mut self, value: bool) -> Self {
        self.optional.fellow_can_fly = Some(value);
        self
    }

    pub fn fellow_people(mut self, value: u8) -> Self {
        self.optional.fellow_people = Some(value);
        self
    }

    pub fn fellow_speed(mut self, value: f32) -> Self {
        self.optional.fellow_speed = Some(value);
        self
    }

    pub fn fellow_adventure_points(mut self, value: u16) -> Self {
        self.optional.fellow_adventure_points = Some(value);
        self
    }

    pub fn fellow_region(mut self, value: Option<SharedString>) -> Self {
        self.optional.fellow_region = value;
        self
    }

    pub fn quality_effect(mut self, value: Option<f32>) -> Self {
        self.optional.quality_effect = value;
        self
    }

    pub fn temper_limit(mut self, value: u8) -> Self {
        self.optional.temper_limit = Some(value);
        self
    }

    pub fn skills(mut self, value: Vec<Skill>) -> Self {
        self.optional.skills = value;
        self
    }

    pub fn max_gem_slots(mut self, value: u8) -> Self {
        self.optional.max_gem_slots = Some(value);
        self
    }

    pub fn reverse_limit(mut self, value: u8) -> Self {
        self.optional.reverse_limit = Some(value);
        self
    }

    pub fn transcendence_limit(mut self, value: u8) -> Self {
        self.optional.transcendence_limit = Some(value);
        self
    }

    pub fn transcendence_effect(mut self, value: Option<f32>) -> Self {
        self.optional.transcendence_effect = value;
        self
    }

    pub fn skill_locale(mut self, value: Option<SharedString>) -> Self {
        self.optional.skill_locale = value;
        self
    }

    pub fn description_locale(mut self, value: Option<SharedString>) -> Self {
        self.optional.description_locale = value;
        self
    }

    pub fn attack(mut self, min: f32, max: f32, attack_speed: f32) -> Self {
        self.optional.attack = Some((min, max, attack_speed));
        self
    }

    pub fn attack_tempering_effect(mut self, value: Option<(f32, f32)>) -> Self {
        self.optional.attack_tempering_effect = value;
        self
    }

    pub fn physic_defense(mut self, value: f32) -> Self {
        self.optional.physic_defense = Some(value);
        self
    }

    pub fn physic_defense_tempering_effect(mut self, value: Option<f32>) -> Self {
        self.optional.physic_defense_tempering_effect = value;
        self
    }

    pub fn magic_defense(mut self, value: f32) -> Self {
        self.optional.magic_defense = Some(value);
        self
    }

    pub fn magic_defense_tempering_effect(mut self, value: Option<f32>) -> Self {
        self.optional.magic_defense_tempering_effect = value;
        self
    }

    pub fn talent_power(mut self, value: u16) -> Self {
        self.optional.talent_power = Some(value);
        self
    }

    pub fn recipe_types(mut self, value: Option<BTreeSet<RecipeType>>) -> Self {
        self.optional.recipe_types = value;
        self
    }

    pub fn recipe_stage(mut self, value: Option<u8>) -> Self {
        self.optional.recipe_stage = value;
        self
    }

    pub fn product(mut self, value: Option<Weak<RefCell<Product>>>) -> Self {
        self.optional.product = value;
        self
    }

    pub fn min_sealed_slots(mut self, value: u8) -> Self {
        self.optional.min_sealed_slots = value;
        self
    }

    pub fn max_sealed_slots(mut self, value: u8) -> Self {
        self.optional.max_sealed_slots = value;
        self
    }

    pub fn min_random_effects(mut self, value: u8) -> Self {
        self.optional.min_random_effects = value;
        self
    }

    pub fn max_random_effects(mut self, value: u8) -> Self {
        self.optional.max_random_effects = value;
        self
    }

    pub fn random_effects(mut self, value: Option<Vec<ItemMinMaxEffect>>) -> Self {
        self.optional.random_effects = value;
        self
    }

    pub fn fellow_stone_effects(
        mut self,
        effects: Vec<ItemMinMaxStepEffect>,
        no_step_effect: Option<ItemMinMaxNoStepEffect>,
        level: u8,
        value: f32,
    ) -> Self {
        self.optional.fellow_stone_effects = Some((effects, no_step_effect, level, value));
        self
    }

    pub fn max_ep(mut self, value: Option<f32>) -> Self {
        self.optional.max_ep = value;
        self
    }

    pub fn random_box_group(mut self, value: Option<Weak<RefCell<RandomBoxGroup>>>) -> Self {
        self.optional.random_box_group = value;
        self
    }

    pub fn package_contents(mut self, value: IndexMap<usize, PackageItem>) -> Self {
        self.optional.package_contents = Some(value);
        self
    }
}

#[derive(Default)]
struct PreviewValues {
    quality: Option<Quality>,
    total_tempering: u8,
    tempering: u8,
    reverse_tempering: u8,
    transcendence: u8,
    random_effects: HashMap<u8, ItemMinMaxEffect>,
    materials: HashMap<u8, bool>,
    linked_recipes_expanded: bool,
}

impl PreviewValues {
    pub fn increase_transcendence(&mut self, transcendence_limit: Option<u8>) {
        if let Some(transcendence_limit) = transcendence_limit {
            self.transcendence = self.transcendence.saturating_add(1).min(transcendence_limit);
        }
    }

    pub fn decrease_transcendence(&mut self) {
        self.transcendence = self.transcendence.saturating_sub(1);
    }

    pub fn increase_tempering(&mut self, temper_limit: Option<u8>, reverse_limit: Option<u8>) {
        if let Some(temper_limit) = temper_limit
            && let Some(reverse_limit) = reverse_limit
        {
            if self.tempering < temper_limit {
                self.tempering = self.tempering.saturating_add(1).min(temper_limit);

                self.total_tempering = self.total_tempering.saturating_add(1).min(temper_limit + reverse_limit);
            }
        }
    }

    pub fn increase_reverse_tempering(&mut self, temper_limit: Option<u8>, reverse_limit: Option<u8>) {
        if let Some(temper_limit) = temper_limit
            && let Some(reverse_limit) = reverse_limit
        {
            if self.reverse_tempering < reverse_limit {
                self.reverse_tempering = self.reverse_tempering.saturating_add(1).min(reverse_limit);

                self.total_tempering = self.total_tempering.saturating_add(1).min(temper_limit + reverse_limit);
            }
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

#[derive(Clone)]
pub enum GameDataLoadingStatus {
    ItemSet,
    SecondaryWeapon,
    Weapon,
    Accessory,
    Armor,
    Tempering,
    Effects,
    Quality,
    Material,
    Recipe,
    ProductMaterial,
    FellowEquip,
    Consume,
    Boost,
    Gem,
    SealedFellow,
    SkillBook,
    Exchange,
    RandomBox,
    RandomBoxGroup,
    RandomBoxProbability,
    Package,
    Style,
    Bag,
    FellowStyle,
    FellowConsume,
    Quest,
    Bracelet,
    Relic,
    FellowBook,
    Event,
    Elluns,
    Fellow,
    Skill,
    Fishing,
    Evolution,
    Synthesis,
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
            GameDataLoadingStatus::Material => t("game-data-loading-material"),
            GameDataLoadingStatus::Recipe => t("game-data-loading-recipe"),
            GameDataLoadingStatus::ProductMaterial => t("game-data-loading-product-material"),
            GameDataLoadingStatus::FellowEquip => t("game-data-loading-fellow-equip"),
            GameDataLoadingStatus::Consume => t("game-data-loading-consume"),
            GameDataLoadingStatus::Boost => t("game-data-loading-boost"),
            GameDataLoadingStatus::Gem => t("game-data-loading-gem"),
            GameDataLoadingStatus::SealedFellow => t("game-data-loading-sealed-fellow"),
            GameDataLoadingStatus::SkillBook => t("game-data-loading-skill-book"),
            GameDataLoadingStatus::Exchange => t("game-data-loading-exchange"),
            GameDataLoadingStatus::RandomBox => t("game-data-loading-random-box"),
            GameDataLoadingStatus::RandomBoxGroup => t("game-data-loading-random-box-group"),
            GameDataLoadingStatus::RandomBoxProbability => t("game-data-loading-random-box-probability"),
            GameDataLoadingStatus::Package => t("game-data-loading-package"),
            GameDataLoadingStatus::Style => t("game-data-loading-style"),
            GameDataLoadingStatus::Bag => t("game-data-loading-bag"),
            GameDataLoadingStatus::FellowStyle => t("game-data-loading-fellow-style"),
            GameDataLoadingStatus::FellowConsume => t("game-data-loading-fellow-consume"),
            GameDataLoadingStatus::Quest => t("game-data-loading-quest"),
            GameDataLoadingStatus::Bracelet => t("game-data-loading-bracelet"),
            GameDataLoadingStatus::Relic => t("game-data-loading-relic"),
            GameDataLoadingStatus::FellowBook => t("game-data-loading-fellow-book"),
            GameDataLoadingStatus::Event => t("game-data-loading-event"),
            GameDataLoadingStatus::Elluns => t("game-data-loading-elluns"),
            GameDataLoadingStatus::Fellow => t("game-data-loading-fellow"),
            GameDataLoadingStatus::Skill => t("game-data-loading-skill"),
            GameDataLoadingStatus::Fishing => t("game-data-loading-fishing"),
            GameDataLoadingStatus::Evolution => t("game-data-loading-evolution"),
            GameDataLoadingStatus::Synthesis => t("game-data-loading-synthesis"),
        }
    }
}

pub struct GameDataView {
    game_data: GameData,
    pub filters: GameDataFilters,
    pub filtered: IndexMap<SharedString, Rc<RefCell<Item>>>,
    is_exporting: bool,
    is_reading: bool,
    explorer_scroll_handle: UniformListScrollHandle,
    tabs_scroll_handle: ScrollHandle,
    focus_handle: FocusHandle,
    debug: bool,
    debug_preview: Entity<EditorState>,
    loading_status: Entity<GameDataLoadingStatus>,
    game_path: Entity<InputState>,
    preview: HashMap<SharedString, PreviewValues>,
    tabs: IndexSet<SharedString>,
    selected_item: Option<SharedString>,
}

impl GameDataView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            game_data: GameData::default(),
            tabs: IndexSet::new(),
            filtered: IndexMap::new(),
            selected_item: None,
            filters: GameDataFilters::new(window, cx),
            is_exporting: false,
            is_reading: false,
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
            debug_preview: cx.new(|cx| EditorState::new(window, cx).language("json")),
        }
    }

    pub fn export_xlsx(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_exporting = true;
        cx.notify();
        if let Ok(path) = cx.app_path() {
            let result = cx.prompt_for_new_path(&path, Some("game_data.xlsx"));

            cx.spawn_in(window, async move |this, cx| {
                if let Some(mut path) = result.await.ok().into_iter().flatten().flatten().next() {
                    if path.extension().is_none_or(|f| f != "xlsx") {
                        path.set_extension("xlsx");
                    }

                    let export_result = (async || -> Result<()> {
                        let game_data = this.read_with(cx, |this, _| this.game_data.clone())?;
                        Ok(game_data.export_xlsx(path.clone()).await?)
                    })()
                    .await;

                    let _ = cx.update({
                        move |window, cx| {
                            match export_result {
                                Ok(_) => window.push_notification(
                                    Notification::new()
                                        .message(t("message-export-completed"))
                                        .with_type(NotificationType::Info)
                                        .action(move |_, _, cx| {
                                            Button::new("notification-open-export").label(t("dialog-button-open")).on_click({
                                                let path = path.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    cx.open_with_system(&path);
                                                    this.dismiss(window, cx);
                                                })
                                            })
                                        }),
                                    cx,
                                ),
                                Err(e) => {
                                    warn!(?e, "Error while export");
                                    window.push_notification((NotificationType::Error, t("message-export-error")), cx);
                                }
                            }
                            let _ = this.update(cx, |this, cx| {
                                this.is_exporting = false;
                                cx.notify();
                            });
                        }
                    });
                } else {
                    let _ = cx.update({
                        move |window, cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.is_exporting = false;
                                cx.notify();
                            });
                        }
                    });
                }
            })
            .detach();
        }
    }

    pub fn apply_filter_and_resort(&mut self) {
        self.filtered = self
            .game_data
            .items
            .iter()
            .filter(|(_, item)| self.filters.check_item(&item.borrow()))
            .map(|(id, item)| (id.clone(), Rc::clone(item)))
            .collect();
    }

    fn render_preview(
        item_type: ItemType,
        preview_builder: PreviewBuilder,
        preview: &PreviewValues,
        items: &IndexMap<SharedString, Rc<RefCell<Item>>>,
        cx: &Context<'_, GameDataView>,
    ) -> Div {
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
                            .when_none(&preview_builder.common.icon, |this| {
                                this.child(
                                    div()
                                        .size(px(128.))
                                        .when_some(preview_builder.common.grade.color(), |this, color| this.border_color(color))
                                        .border_2(),
                                )
                            })
                            .when_some(preview_builder.common.icon.as_ref(), |this, icon| {
                                this.child(
                                    img(ImageSource::Image(icon.clone()))
                                        .object_fit(ObjectFit::Cover)
                                        .size(px(128.))
                                        .border_2()
                                        .when_some(preview_builder.common.grade.color(), |this, color| this.border_color(color)),
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
                                            .child(preview_builder.common.get_localized_name())
                                            .text_lg()
                                            .when_some(preview_builder.common.grade.color(), |this, color| this.text_color(color)),
                                    )
                                    .child(
                                        Button::new("copy-item-name")
                                            .icon(IconName::Copy)
                                            .ghost()
                                            .compact()
                                            .on_click(cx.listener({
                                                let item_locale = preview_builder.common.get_localized_name();
                                                move |_, _, window, cx| {
                                                    cx.write_to_clipboard(ClipboardItem::new_string(item_locale.to_string()));
                                                    window.push_notification((NotificationType::Info, t("message-copy-item-name")), cx);
                                                }
                                            })),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(preview_builder.common.grade.locale())
                                    .when_some(preview_builder.common.grade.color(), |this, color| this.text_color(color))
                                    .when_some(preview.quality, |this, quality| {
                                        this.child(
                                            Button::new("button-quality")
                                                .small()
                                                .link()
                                                .when_some(preview_builder.common.grade.color(), |this, color| this.text_color(color))
                                                .label(quality.locale())
                                                .on_click(cx.listener({
                                                    let id = preview_builder.common.id.clone();
                                                    move |this, _, _, cx| {
                                                        this.preview.entry(id.clone()).and_modify(|v| {
                                                            v.quality = Some(quality.next());
                                                        });
                                                        cx.notify();
                                                    }
                                                })),
                                        )
                                    }),
                            )
                            .when_some(preview_builder.optional.max_ep, |this, max_ep| {
                                this.child(h_flex().gap_1().child(format!("{} {:.1}", t("item-effect-max-ep"), max_ep)))
                            })
                            .when_some(preview_builder.optional.attack, |this, (min, max, attack_speed)| {
                                let dps = (max
                                    + preview_builder.optional.quality_effect.unwrap_or_default()
                                    + min
                                    + preview_builder.optional.quality_effect.unwrap_or_default())
                                    / 2.0
                                    / attack_speed;
                                this.child(h_flex().gap_1().child(format!("{} {:.1}", t("item-attack-dps"), dps)).when_some(
                                    preview_builder.optional.attack_tempering_effect,
                                    |this, (dps_tempering_effect, _)| {
                                        this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", dps_tempering_effect)))
                                    },
                                ))
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(format!(
                                            "{} {} - {}",
                                            t("item-attack"),
                                            min + preview_builder.optional.quality_effect.unwrap_or_default(),
                                            max + preview_builder.optional.quality_effect.unwrap_or_default()
                                        ))
                                        .when_some(preview_builder.optional.attack_tempering_effect, |this, (_, min_max_tempering_effect)| {
                                            this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", min_max_tempering_effect)))
                                        }),
                                )
                            })
                            .when_some(preview_builder.optional.physic_defense, |this, physic_defense| {
                                this.child(
                                    h_flex()
                                        .gap_1()
                                        .child(format!(
                                            "{} {:.1}",
                                            t("item-physical-defense"),
                                            physic_defense + preview_builder.optional.quality_effect.unwrap_or_default()
                                        ))
                                        .when_some(preview_builder.optional.physic_defense_tempering_effect, |this, tempering_effect| {
                                            this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", tempering_effect)))
                                        }),
                                )
                            })
                            .when_some(preview_builder.optional.magic_defense, |this, magic_defense| {
                                this.child(
                                    h_flex()
                                        .gap_1()
                                        .child(format!("{} {:.1}", t("item-magic-defense"), magic_defense))
                                        .when_some(preview_builder.optional.magic_defense_tempering_effect, |this, tempering_effect| {
                                            this.child(div().text_color(cx.theme().cyan).child(format!("({:+.1})", tempering_effect)))
                                        }),
                                )
                            })
                            .when_some(preview_builder.optional.attack, |this, (_, _, attack_speed)| {
                                this.child(format!("{} {:.1}", t("item-attack-speed"), attack_speed))
                            })
                            .when_some(preview_builder.optional.talent_power, |this, talent_power| {
                                this.child(format!("{} {}", t("item-talent-power"), talent_power))
                            })
                            .when_some(preview_builder.optional.max_gem_slots, |this, max_gem_slots| {
                                this.child(format!("{} {}", t("item-max-gem-slots"), max_gem_slots))
                            })
                            .when_some(preview_builder.optional.fellow_type, |this, fellow_type| {
                                this.child(
                                    h_flex()
                                        .gap_1()
                                        .child(fellow_type.localized())
                                        .when(fellow_type == FellowType::Mount, |this| {
                                            this.when_some(preview_builder.optional.fellow_can_fly, |this, can_fly| {
                                                this.when_else(
                                                    can_fly,
                                                    |this| this.child(t("item-fly-mount")),
                                                    |this| this.child(t("item-land-mount")),
                                                )
                                            })
                                            .when_some(preview_builder.optional.fellow_people, |this, fellow_people| {
                                                this.child(t_v("item-fellow-people", vec![("value", fellow_people)]))
                                            })
                                        }),
                                )
                            })
                            .when_some(preview_builder.optional.fellow_region, |this, fellow_region| this.child(fellow_region))
                            .when_some(preview_builder.optional.fellow_adventure_points, |this, fellow_adventure_points| {
                                this.child(format!("{} {}", t("item-adventure-points"), fellow_adventure_points))
                            })
                            .when_some(preview_builder.optional.fellow_speed, |this, fellow_speed| {
                                this.child(format!("{} {:.2}", t("item-speed"), fellow_speed))
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_start()
                    .child(
                        v_flex()
                            .when_some(preview_builder.optional.transcendence_limit, |this, transcendence_limit| {
                                this.when_else(
                                    transcendence_limit == 0,
                                    |this| this.child(t("item-no-transcendence")),
                                    |this| this.child(t("item-transcendence-limit")),
                                )
                            })
                            .when_some(preview_builder.optional.temper_limit, |this, temper_limit| {
                                this.when_else(
                                    temper_limit == 0,
                                    |this| this.child(t("item-no-tempering")),
                                    |this| this.child(t("item-tempering-limit")),
                                )
                            })
                            .when_some(preview_builder.optional.reverse_limit, |this, reverse_limit| {
                                this.when_else(
                                    reverse_limit == 0,
                                    |this| this.child(t("item-no-reverse-tempering")),
                                    |this| this.child(t("item-reverse-tempering-limit")),
                                )
                            }),
                    )
                    .child(
                        v_flex()
                            .justify_start()
                            .when_some(preview_builder.optional.transcendence_limit, |this, transcendence_limit| {
                                this.when_else(
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
                                                        .on_click(cx.listener({
                                                            let id = preview_builder.common.id.clone();
                                                            move |this, _, _, cx| {
                                                                let preview =
                                                                    this.preview.entry(id.clone()).or_insert_with(|| PreviewValues::default());
                                                                preview.decrease_transcendence();
                                                                cx.notify();
                                                            }
                                                        })),
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
                                                        .on_click(cx.listener({
                                                            let id = preview_builder.common.id.clone();
                                                            move |this, _, _, cx| {
                                                                let preview =
                                                                    this.preview.entry(id.clone()).or_insert_with(|| PreviewValues::default());
                                                                preview.increase_transcendence(preview_builder.optional.transcendence_limit);

                                                                cx.notify();
                                                            }
                                                        })),
                                                ),
                                        )
                                    },
                                )
                            })
                            .when_some(preview_builder.optional.temper_limit, |this, temper_limit| {
                                this.when_else(
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
                                                        .on_click(cx.listener({
                                                            let id = preview_builder.common.id.clone();
                                                            move |this, _, _, cx| {
                                                                let preview =
                                                                    this.preview.entry(id.clone()).or_insert_with(|| PreviewValues::default());
                                                                preview.decrease_tempering();
                                                                cx.notify();
                                                            }
                                                        })),
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
                                                        .on_click(cx.listener({
                                                            let id = preview_builder.common.id.clone();
                                                            move |this, _, _, cx| {
                                                                let preview =
                                                                    this.preview.entry(id.clone()).or_insert_with(|| PreviewValues::default());
                                                                preview.increase_tempering(
                                                                    preview_builder.optional.temper_limit,
                                                                    preview_builder.optional.reverse_limit,
                                                                );

                                                                cx.notify();
                                                            }
                                                        })),
                                                ),
                                        )
                                    },
                                )
                            })
                            .when_some(preview_builder.optional.reverse_limit, |this, reverse_limit| {
                                this.when_else(
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
                                                        .on_click(cx.listener({
                                                            let id = preview_builder.common.id.clone();
                                                            move |this, _, _, cx| {
                                                                let preview =
                                                                    this.preview.entry(id.clone()).or_insert_with(|| PreviewValues::default());
                                                                preview.decrease_reverse_tempering();
                                                                cx.notify();
                                                            }
                                                        })),
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
                                                        .on_click(cx.listener({
                                                            let id = preview_builder.common.id.clone();
                                                            move |this, _, _, cx| {
                                                                let preview =
                                                                    this.preview.entry(id.clone()).or_insert_with(|| PreviewValues::default());
                                                                preview.increase_reverse_tempering(
                                                                    preview_builder.optional.temper_limit,
                                                                    preview_builder.optional.reverse_limit,
                                                                );

                                                                cx.notify();
                                                            }
                                                        })),
                                                ),
                                        )
                                    },
                                )
                            }),
                    ),
            )
            .when(preview_builder.common.required_level != 0, |this| {
                this.child(format!("{}: {}", t("item-required-level"), preview_builder.common.required_level))
            })
            .when(preview_builder.optional.max_sealed_slots > 0, |this| {
                this.child(format!(
                    "{}: {} - {}",
                    t("item-sealed-stones-slots"),
                    preview_builder.optional.min_sealed_slots,
                    preview_builder.optional.max_sealed_slots
                ))
            })
            .when(!preview_builder.common.usable_class.is_empty(), |this| {
                this.child(h_flex().gap_1().children(preview_builder.common.usable_class.iter().map(|c| c.locale())))
            })
            .child(
                h_flex()
                    .gap_1()
                    .when_some(preview_builder.common.binding.and_then(|t| t.locale()), |this, locale| {
                        this.child(div().text_color(cx.theme().yellow).child(locale))
                    })
                    .when(preview_builder.common.no_trade, |this| this.child(t("item-tag-no-trade")))
                    .when(preview_builder.common.no_sell, |this| this.child(t("item-tag-no-sell")))
                    .when(preview_builder.common.no_destroy, |this| this.child(t("item-tag-no-destroy"))),
            )
            .when_some(preview_builder.optional.recipe_types, |this, recipe_types| {
                this.child(
                    h_flex()
                        .gap_1()
                        .children(recipe_types.iter().map(|m| m.locale()))
                        .when_some(preview_builder.optional.recipe_stage, |this, recipe_stage| {
                            this.child(t_v("item-recipe-stage", vec![("stage", recipe_stage)]))
                        }),
                )
            })
            .when_some(preview_builder.optional.skill_locale, |this, skill_locale| {
                this.child(div().mt_2().text_color(cx.theme().success).child(t("item-equipped-skill")))
                    .child(RichText::parse(&skill_locale, cx.theme().yellow))
            })
            .when(!preview_builder.common.evolution.is_empty(), {
                move |this| {
                    this.child(div().mt_2().text_color(cx.theme().success).child(t("item-evolution")))
                        .child(v_flex().gap_1().children(preview_builder.common.evolution.iter().enumerate().map({
                            move |(index, evolution)| {
                                let (is_present, chance, node_id, count, grade, icon, name) = (
                                    evolution.material_fellow.item.is_some(),
                                    evolution.successrate,
                                    evolution.material_fellow.id.clone(),
                                    evolution.material_count,
                                    evolution
                                        .material_fellow
                                        .item
                                        .as_ref()
                                        .and_then(|f| f.upgrade())
                                        .map(|m| m.borrow().get_grade()),
                                    evolution
                                        .material_fellow
                                        .item
                                        .as_ref()
                                        .and_then(|f| f.upgrade())
                                        .and_then(|m| m.borrow().get_icon()),
                                    evolution
                                        .material_fellow
                                        .item
                                        .as_ref()
                                        .and_then(|f| f.upgrade())
                                        .map(|m| m.borrow().get_localized_name())
                                        .unwrap_or_else(|| evolution.material_fellow.id.clone()),
                                );

                                h_flex()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .relative()
                                            .id(format!("icon-evolution-{index}"))
                                            .when_none(&icon, |this| {
                                                this.child(
                                                    div()
                                                        .size(px(40.))
                                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color))
                                                        .border_2(),
                                                )
                                            })
                                            .when_some(icon, |this, icon| {
                                                this.child(
                                                    img(ImageSource::Image(icon))
                                                        .object_fit(ObjectFit::Cover)
                                                        .size(px(40.))
                                                        .border_2()
                                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color)),
                                                )
                                            }),
                                    )
                                    .child(
                                        v_flex()
                                            .items_start()
                                            .child(
                                                Button::new(format!("button-{}", node_id))
                                                    .label(format!("x{count} {}", name))
                                                    .link()
                                                    .small()
                                                    .mb_1()
                                                    .text_color(cx.theme().foreground)
                                                    .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                                    .when(is_present, |this| {
                                                        this.on_click(cx.listener({
                                                            move |this, _, window, cx| {
                                                                this.tabs.insert(node_id.clone());
                                                                this.set_selected_item(Some(node_id.clone()), window, cx);
                                                                cx.notify();
                                                            }
                                                        }))
                                                    }),
                                            )
                                            .child(t_v("item-effect-evolution-chance-percent", vec![("value", format!("{:.0}", chance))])),
                                    )
                            }
                        })))
                }
            })
            .when(!preview_builder.common.synthesis_fellows.is_empty(), {
                move |this| {
                    this.child(
                        v_flex()
                            .gap_1()
                            .children(preview_builder.common.synthesis_fellows.iter().enumerate().map(|(_, f)| {
                                div()
                                    .text_color(cx.theme().success)
                                    .child(t_v("item-synthesis-chance", vec![("value", Decimal::from_f32(f.rate).unwrap().to_string())]))
                            })),
                    )
                }
            })
            .when(!preview_builder.common.synthesis_parts.is_empty(), {
                move |this| {
                    this.child(div().mt_2().text_color(cx.theme().success).child(t("item-synthesis")))
                        .child(v_flex().gap_1().children(preview_builder.common.synthesis_parts.iter().enumerate().map({
                            move |(index, synthesis_parts)| {
                                let (is_present, chance, node_id, count, grade, icon, name) = (
                                    synthesis_parts.itemid.item.is_some(),
                                    synthesis_parts.successrate,
                                    synthesis_parts.itemid.id.clone(),
                                    synthesis_parts.itemcnt,
                                    synthesis_parts
                                        .itemid
                                        .item
                                        .as_ref()
                                        .and_then(|f| f.upgrade())
                                        .map(|m| m.borrow().get_grade()),
                                    synthesis_parts
                                        .itemid
                                        .item
                                        .as_ref()
                                        .and_then(|f| f.upgrade())
                                        .and_then(|m| m.borrow().get_icon()),
                                    synthesis_parts
                                        .itemid
                                        .item
                                        .as_ref()
                                        .and_then(|f| f.upgrade())
                                        .map(|m| m.borrow().get_localized_name())
                                        .unwrap_or_else(|| synthesis_parts.itemid.id.clone()),
                                );

                                h_flex()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .relative()
                                            .id(format!("icon-synthesis-parts-{index}"))
                                            .when_none(&icon, |this| {
                                                this.child(
                                                    div()
                                                        .size(px(40.))
                                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color))
                                                        .border_2(),
                                                )
                                            })
                                            .when_some(icon, |this, icon| {
                                                this.child(
                                                    img(ImageSource::Image(icon))
                                                        .object_fit(ObjectFit::Cover)
                                                        .size(px(40.))
                                                        .border_2()
                                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color)),
                                                )
                                            }),
                                    )
                                    .child(
                                        v_flex()
                                            .items_start()
                                            .child(
                                                Button::new(format!("button-{}", node_id))
                                                    .label(format!("x{count} {}", name))
                                                    .link()
                                                    .small()
                                                    .mb_1()
                                                    .text_color(cx.theme().foreground)
                                                    .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                                    .when(is_present, |this| {
                                                        this.on_click(cx.listener({
                                                            move |this, _, window, cx| {
                                                                this.tabs.insert(node_id.clone());
                                                                this.set_selected_item(Some(node_id.clone()), window, cx);
                                                                cx.notify();
                                                            }
                                                        }))
                                                    }),
                                            )
                                            .child(t_v("item-effect-synthesis-chance-percent", vec![("value", format!("{:.0}", chance))])),
                                    )
                            }
                        })))
                }
            })
            .when(preview_builder.optional.max_random_effects > 0, {
                let id = preview_builder.common.id.clone();
                move |this| {
                    this.child(div().mt_2().text_color(cx.theme().yellow).child(format!(
                        "{} - {} {}",
                        preview_builder.optional.min_random_effects,
                        preview_builder.optional.max_random_effects,
                        t("item-random-equipped-effects")
                    )))
                    .when_some(preview_builder.optional.random_effects, move |this, random_effects| {
                        this.children((0..preview_builder.optional.max_random_effects).map(|i| {
                            let re = preview.random_effects.get(&i);

                            Button::new(format!("random-effects-dropdown-{}", i))
                                .small()
                                .text()
                                .my_1()
                                .cursor_pointer()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .when_some(re, |this, re| this.text_color(cx.theme().success).child(re.get_locale()))
                                        .when_none(&re, |this| {
                                            this.text_color(cx.theme().foreground).child(t("label-select-random-equipped-effect"))
                                        })
                                        .when_some(
                                            preview_builder.optional.transcendence_effect.and_then(|transcendence_effect| {
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
                                    let viewer_entity = cx.entity();
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
            .when(!preview_builder.common.effects.is_empty(), |this| {
                this.child(div().mt_2().text_color(cx.theme().yellow).child(t("item-equipped-effects")))
                    .children(preview_builder.common.effects.iter().map(|equip_effect| {
                        h_flex()
                            .gap_1()
                            .child(div().text_color(cx.theme().success).child(equip_effect.get_locale()))
                            .when_some(
                                preview_builder.optional.transcendence_effect.and_then(|transcendence_effect| {
                                    equip_effect
                                        .parsed
                                        .as_ref()
                                        .map(|(key, effect_value)| (key, effect_value * transcendence_effect))
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
                            )
                    }))
            })
            .when_some(preview_builder.common.item_set.as_ref(), |this, item_set| {
                this.child(v_flex().child(div().mt_2().text_color(cx.theme().yellow).child(item_set.get_localized_name())))
                    .children(item_set.items.iter().map(|item| items.get(item)).map(|item| {
                        v_flex().items_start().when_some(item, {
                            move |this, item| {
                                let id = item.borrow().get_id();
                                let grade = item.borrow().get_grade();
                                this.child(
                                    Button::new(format!("button-{}", id))
                                        .label(item.borrow().get_localized_name())
                                        .link()
                                        .small()
                                        .mb_2()
                                        .when_some(grade.color(), |this, color| this.text_color(color))
                                        .on_click(cx.listener({
                                            move |this, _, window, cx| {
                                                this.tabs.insert(id.clone());
                                                this.set_selected_item(Some(id.clone()), window, cx);
                                                cx.notify();
                                            }
                                        })),
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
                                .when_some(set_effect.get_localized_name(), |this, skill| {
                                    this.child(RichText::parse(&skill, cx.theme().yellow))
                                })
                        })))
                    })
            })
            .when_some(
                preview_builder.optional.fellow_stone_effects,
                |this, (effects, plus, tempered, tempered_effect)| {
                    this.child(
                        h_flex()
                            .mt_2()
                            .text_color(cx.theme().success)
                            .gap_2()
                            .items_start()
                            .child(
                                v_flex()
                                    .child(div().text_color(cx.theme().yellow).child(t("item-equipped-effects")))
                                    .children(effects.iter().map(|e| {
                                        e.parsed
                                            .as_ref()
                                            .map(|f| SharedString::new(t_v(&f.0, vec![("value", "")]).trim_end_matches(&['-']).trim_end()))
                                            .unwrap_or_else(|| e.effect.clone())
                                    }))
                                    .when_some(plus.as_ref(), |this, e| {
                                        this.child(
                                            div().text_color(cx.theme().cyan).child(
                                                e.parsed
                                                    .as_ref()
                                                    .map(|f| SharedString::new(t_v(&f.0, vec![("value", "")]).trim_end_matches(&['-']).trim_end()))
                                                    .unwrap_or_else(|| e.effect.clone()),
                                            ),
                                        )
                                    }),
                            )
                            .child(
                                v_flex()
                                    .child(div().text_color(cx.theme().yellow).child(t("sealed-fellow-min-level")))
                                    .children(effects.iter().map(|e| {
                                        e.parsed
                                            .as_ref()
                                            .map(|(key, min, max, _)| {
                                                if key.ends_with("-minus-percent") {
                                                    format!("{:-.2}% ~ -{:.2}%", min, max)
                                                } else if key.ends_with("-percent") {
                                                    format!("{:+.2}% ~ {:+.2}%", min, max)
                                                } else {
                                                    format!("{:+.0} ~ {:+.0}", min, max)
                                                }
                                            })
                                            .unwrap_or_default()
                                    })),
                            )
                            .child(
                                v_flex()
                                    .child(div().text_color(cx.theme().yellow).child(t("sealed-fellow-max-level")))
                                    .children(effects.iter().map(|e| {
                                        e.parsed
                                            .as_ref()
                                            .map(|(key, min, max, step)| {
                                                if key.ends_with("-minus-percent") {
                                                    format!("{:-.2}% ~ -{:.2}%", min + step, max + step)
                                                } else if key.ends_with("-percent") {
                                                    format!("{:+.2}% ~ {:+.2}%", min + step, max + step)
                                                } else {
                                                    format!("{:+.0} ~ {:+.0}", min + step, max + step)
                                                }
                                            })
                                            .unwrap_or_default()
                                    })),
                            )
                            .when(plus.is_some(), |this| {
                                this.child(
                                    v_flex()
                                        .child(div().text_color(cx.theme().yellow).child(t("sealed-fellow-plus-level")))
                                        .children(effects.iter().map(|e| {
                                            e.parsed
                                                .as_ref()
                                                .map(|(key, min, max, step)| {
                                                    if key.ends_with("-minus-percent") {
                                                        format!("{:-.2}% ~ -{:.2}%", min + step, max + step)
                                                    } else if key.ends_with("-percent") {
                                                        format!("{:+.2}% ~ {:+.2}%", min + step, max + step)
                                                    } else {
                                                        format!("{:+.0} ~ {:+.0}", min + step, max + step)
                                                    }
                                                })
                                                .unwrap_or_default()
                                        }))
                                        .when_some(plus.as_ref(), |this, e| {
                                            this.child(
                                                div().text_color(cx.theme().cyan).child(
                                                    e.parsed
                                                        .as_ref()
                                                        .map(|(key, min, max)| {
                                                            if key.ends_with("-minus-percent") {
                                                                format!("{:-.2}% ~ -{:.2}%", min, max)
                                                            } else if key.ends_with("-percent") {
                                                                format!("{:+.2}% ~ {:+.2}%", min, max)
                                                            } else {
                                                                format!("{:+.0} ~ {:+.0}", min, max)
                                                            }
                                                        })
                                                        .unwrap_or_default(),
                                                ),
                                            )
                                        }),
                                )
                            })
                            .when(tempered != 0, |this| {
                                this.child(
                                    v_flex()
                                        .child(
                                            div()
                                                .text_color(cx.theme().yellow)
                                                .child(t_v("sealed-fellow-tempered-level", vec![("level", tempered)])),
                                        )
                                        .children(effects.iter().map(|e| {
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    e.parsed
                                                        .as_ref()
                                                        .map(|(key, min, max, step)| {
                                                            if key.ends_with("-minus-percent") {
                                                                format!(
                                                                    "{:-.2}% ~ -{:.2}%",
                                                                    (min + step) * (1.0 + tempered_effect / 100.0),
                                                                    (max + step) * (1.0 + tempered_effect / 100.0)
                                                                )
                                                            } else if key.ends_with("-percent") {
                                                                format!(
                                                                    "{:+.2}% ~ {:+.2}%",
                                                                    (min + step) * (1.0 + tempered_effect / 100.0),
                                                                    (max + step) * (1.0 + tempered_effect / 100.0)
                                                                )
                                                            } else {
                                                                format!(
                                                                    "{:+.0} ~ {:+.0}",
                                                                    (min + step) * (1.0 + tempered_effect / 100.0),
                                                                    (max + step) * (1.0 + tempered_effect / 100.0)
                                                                )
                                                            }
                                                        })
                                                        .unwrap_or_default(),
                                                )
                                                .when(e.parsed.is_some(), |this| {
                                                    this.child(div().text_color(cx.theme().yellow).child(format!("({:+.0}%)", tempered_effect)))
                                                })
                                        }))
                                        .when_some(plus.as_ref(), |this, e| {
                                            this.child(
                                                h_flex()
                                                    .text_color(cx.theme().cyan)
                                                    .gap_1()
                                                    .child(
                                                        e.parsed
                                                            .as_ref()
                                                            .map(|(key, min, max)| {
                                                                if key.ends_with("-minus-percent") {
                                                                    format!(
                                                                        "{:-.2}% ~ -{:.2}%",
                                                                        min * (1.0 + tempered_effect / 100.0),
                                                                        max * (1.0 + tempered_effect / 100.0)
                                                                    )
                                                                } else if key.ends_with("-percent") {
                                                                    format!(
                                                                        "{:+.2}% ~ {:+.2}%",
                                                                        min * (1.0 + tempered_effect / 100.0),
                                                                        max * (1.0 + tempered_effect / 100.0)
                                                                    )
                                                                } else {
                                                                    format!(
                                                                        "{:+.0} ~ {:+.0}",
                                                                        min * (1.0 + tempered_effect / 100.0),
                                                                        max * (1.0 + tempered_effect / 100.0)
                                                                    )
                                                                }
                                                            })
                                                            .unwrap_or_default(),
                                                    )
                                                    .when(e.parsed.is_some(), |this| {
                                                        this.child(div().text_color(cx.theme().yellow).child(format!("({:+.0}%)", tempered_effect)))
                                                    }),
                                            )
                                        }),
                                )
                            }),
                    )
                },
            )
            .when(!preview_builder.optional.skills.is_empty(), |this| {
                this.when(item_type == ItemType::Fellow, |this| {
                    this.child(div().mt_2().text_color(cx.theme().success).child(t("item-skills")))
                })
                .children(preview_builder.optional.skills.iter().map(|skill| {
                    let last_level_skill_data = if item_type == ItemType::Fellow {
                        skill.skill_data.skill_level.iter().rev().find(|f| f.learn_level <= skill.max_item_level)
                    } else {
                        skill.skill_data.skill_level.iter().find(|f| f.level == skill.max_item_level)
                    };
                    v_flex()
                        .mt_2()
                        .when(item_type == ItemType::Fellow, |this| {
                            this.child(
                                h_flex()
                                    .gap_2()
                                    .items_start()
                                    .when_some(skill.icon.as_ref(), |this, icon| {
                                        this.child(img(ImageSource::Image(icon.clone())).object_fit(ObjectFit::Cover).size(px(64.)))
                                    })
                                    .child(
                                        v_flex()
                                            .child(div().font_bold().child(skill.get_localized_name()))
                                            .when_else(
                                                skill.passive_skill == 1,
                                                |this| this.child(t("item-skill-passive")),
                                                |this| this.child(t("item-skill-active")),
                                            )
                                            .when(skill.cool_time != 0, |this| {
                                                this.child(t_v("item-skill-cooldown", vec![("value", skill.cool_time / 1000)]))
                                            }),
                                    ),
                            )
                            .when_some(skill.get_localized_description(), |this, description| {
                                this.child(div().mt_2().child(RichText::parse(&description, cx.theme().yellow)))
                            })
                        })
                        .when_some(last_level_skill_data, |this, s| {
                            this.when_some(s.buff1.effect_pattern_list.as_ref(), |this, buff| {
                                this.when_some(buff.effect_pattern.as_ref(), |this, effects| {
                                    this.when(!effects.is_empty(), |this| {
                                        this.child(div().text_color(cx.theme().yellow).mt_2().child(t("item-skill-effects")))
                                            .children(effects.iter().map(|e| {
                                                div()
                                                    .text_color(cx.theme().success)
                                                    .child(e.effect.get_locale_with_duration(s.keep_buff_time))
                                            }))
                                    })
                                })
                            })
                        })
                }))
            })
            .when_some(preview_builder.optional.description_locale, |this, description_locale| {
                this.child(div().mt_2().child(RichText::parse(&description_locale, cx.theme().yellow)))
            })
            .when_some(preview_builder.optional.product.and_then(|f| f.upgrade()).map(|f| f.borrow().clone()), {
                let id = preview_builder.common.id.clone();
                move |this, product| {
                    let item = product.node.item.clone();

                    let icon = item.as_ref().and_then(|f| f.upgrade()).and_then(|i| i.borrow().get_icon());
                    let grade = item.as_ref().and_then(|f| f.upgrade()).map(|i| i.borrow().get_grade());
                    let chance = product.success_probability;
                    let inheritance_enhancement_condition = product.inheritance_enhancement_condition;
                    let inheritance_transcendence_condition = product.inheritance_transcendence_condition;

                    this.child(
                        v_flex()
                            .gap_1()
                            .child(div().mt_2().text_color(cx.theme().success).child(t("item-product-result")))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        h_flex()
                                            .when_none(&icon, |this| {
                                                this.child(
                                                    div()
                                                        .size(px(64.))
                                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color))
                                                        .border_2(),
                                                )
                                            })
                                            .when_some(icon, |this, icon| {
                                                this.child(
                                                    img(ImageSource::Image(icon))
                                                        .object_fit(ObjectFit::Cover)
                                                        .size(px(64.))
                                                        .border_2()
                                                        .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color)),
                                                )
                                            }),
                                    )
                                    .child(
                                        v_flex()
                                            .items_start()
                                            .child(
                                                Button::new(format!("button-{}", product.node.id))
                                                    .label(
                                                        item.as_ref()
                                                            .and_then(|f| f.upgrade())
                                                            .map(|i| i.borrow().get_localized_name())
                                                            .unwrap_or_else(|| product.node.id.clone()),
                                                    )
                                                    .link()
                                                    .small()
                                                    .mb_2()
                                                    .text_color(cx.theme().foreground)
                                                    .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                                    .when(product.inheritance_on_craft, move |this| {
                                                        this.child(
                                                            div()
                                                                .id("item-product-inheritance")
                                                                .text_color(cx.theme().yellow)
                                                                .tooltip(move |window, cx| {
                                                                    Tooltip::new(t_v(
                                                                        "item-product-inheritance-conditions",
                                                                        vec![
                                                                            ("tempering", format!("{:+}", inheritance_enhancement_condition)),
                                                                            ("transcendence", format!("{:+}", inheritance_transcendence_condition)),
                                                                        ],
                                                                    ))
                                                                    .build(window, cx)
                                                                })
                                                                .child(t("item-product-inheritance")),
                                                        )
                                                    })
                                                    .when(item.is_some(), |this| {
                                                        this.on_click(cx.listener({
                                                            move |this, _, window, cx| {
                                                                this.tabs.insert(product.node.id.clone());
                                                                this.set_selected_item(Some(product.node.id.clone()), window, cx);
                                                                cx.notify();
                                                            }
                                                        }))
                                                    }),
                                            )
                                            .child(t_v("item-effect-crafting-chance-percent", vec![("value", format!("{:.0}", chance))])),
                                    ),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().mt_2().text_color(cx.theme().success).child(t("item-product-materials")))
                            .children(product.materials.into_iter().map({
                                move |(index, material)| {
                                    let use_additional = preview.materials.get(&(index as u8)).cloned().unwrap_or_default();
                                    let (is_present, node_id, count, grade, icon, name) = if !use_additional {
                                        (
                                            material.node.item.is_some(),
                                            material.node.id.clone(),
                                            material.count,
                                            material.node.item.as_ref().and_then(|f| f.upgrade()).map(|m| m.borrow().get_grade()),
                                            material.node.item.as_ref().and_then(|f| f.upgrade()).and_then(|m| m.borrow().get_icon()),
                                            material
                                                .node
                                                .item
                                                .as_ref()
                                                .and_then(|f| f.upgrade())
                                                .map(|m| m.borrow().get_localized_name())
                                                .unwrap_or_else(|| material.node.id.clone()),
                                        )
                                    } else {
                                        (
                                            material.additional_node.as_ref().and_then(|f| f.item.as_ref()).is_some(),
                                            material.additional_node.as_ref().unwrap().id.clone(),
                                            material.additional_count,
                                            material
                                                .additional_node
                                                .as_ref()
                                                .and_then(|f| f.item.as_ref())
                                                .and_then(|f| f.upgrade())
                                                .map(|m| m.borrow().get_grade()),
                                            material
                                                .additional_node
                                                .as_ref()
                                                .and_then(|f| f.item.as_ref())
                                                .and_then(|f| f.upgrade())
                                                .and_then(|m| m.borrow().get_icon()),
                                            material
                                                .additional_node
                                                .as_ref()
                                                .and_then(|f| f.item.as_ref())
                                                .and_then(|f| f.upgrade())
                                                .map(|m| m.borrow().get_localized_name())
                                                .unwrap_or_else(|| material.additional_node.as_ref().unwrap().id.clone()),
                                        )
                                    };

                                    h_flex()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .relative()
                                                .id(format!("icon-additional-{index}"))
                                                .when_none(&icon, |this| {
                                                    this.child(
                                                        div()
                                                            .size(px(40.))
                                                            .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color))
                                                            .border_2(),
                                                    )
                                                })
                                                .when_some(icon, |this, icon| {
                                                    this.child(
                                                        img(ImageSource::Image(icon))
                                                            .object_fit(ObjectFit::Cover)
                                                            .size(px(40.))
                                                            .border_2()
                                                            .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color)),
                                                    )
                                                })
                                                .when(material.additional_node.is_some(), {
                                                    let id = id.clone();
                                                    move |this| {
                                                        this.child(
                                                            div()
                                                                .flex()
                                                                .justify_center()
                                                                .items_center()
                                                                .bg(cx.theme().blue)
                                                                .bottom(px(0.0))
                                                                .right(px(0.0))
                                                                .size(px(16.0))
                                                                .absolute()
                                                                .child(Icon::new(AppIcon::Repeat).text_color(rgb(0xffffff))),
                                                        )
                                                        .on_click(cx.listener({
                                                            move |this, _, _, cx| {
                                                                this.preview.entry(id.clone()).and_modify(|v| {
                                                                    v.materials.insert(index as u8, !use_additional);
                                                                });
                                                                cx.notify();
                                                            }
                                                        }))
                                                    }
                                                }),
                                        )
                                        .child(
                                            Button::new(format!("button-{}", node_id))
                                                .label(format!("x{count} {}", name))
                                                .link()
                                                .small()
                                                .mb_2()
                                                .text_color(cx.theme().foreground)
                                                .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                                .when(is_present, |this| {
                                                    this.on_click(cx.listener({
                                                        move |this, _, window, cx| {
                                                            this.tabs.insert(node_id.clone());
                                                            this.set_selected_item(Some(node_id.clone()), window, cx);
                                                            cx.notify();
                                                        }
                                                    }))
                                                }),
                                        )
                                }
                            })),
                    )
                }
            })
            .when_some(
                preview_builder
                    .optional
                    .random_box_group
                    .and_then(|f| f.upgrade())
                    .map(|f| f.borrow().clone()),
                {
                    move |this, random_box_group| {
                        let contents = random_box_group.items;

                        this.child(
                            v_flex()
                                .gap_1()
                                .child(div().mt_2().text_color(cx.theme().success).child(t("item-random-box-contents")))
                                .children(contents.into_iter().map({
                                    move |(index, node)| {
                                        let (is_present, node_id, grade, icon, name) = {
                                            (
                                                node.item.is_some(),
                                                node.id.clone(),
                                                node.item.as_ref().and_then(|f| f.upgrade()).map(|m| m.borrow().get_grade()),
                                                node.item.as_ref().and_then(|f| f.upgrade()).and_then(|m| m.borrow().get_icon()),
                                                node.item
                                                    .as_ref()
                                                    .and_then(|f| f.upgrade())
                                                    .map(|m| m.borrow().get_localized_name())
                                                    .unwrap_or_else(|| node.id.clone()),
                                            )
                                        };
                                        let probability = random_box_group.probabilities.get(&index);
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                h_flex()
                                                    .relative()
                                                    .id(format!("icon-additional-{index}"))
                                                    .when_none(&icon, |this| {
                                                        this.child(
                                                            div()
                                                                .size(px(40.))
                                                                .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color))
                                                                .border_2(),
                                                        )
                                                    })
                                                    .when_some(icon, |this, icon| {
                                                        this.child(
                                                            img(ImageSource::Image(icon))
                                                                .object_fit(ObjectFit::Cover)
                                                                .size(px(40.))
                                                                .border_2()
                                                                .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color)),
                                                        )
                                                    }),
                                            )
                                            .child(
                                                v_flex()
                                                    .items_start()
                                                    .child(
                                                        Button::new(format!("button-{}-{}", index, node_id))
                                                            .label(name)
                                                            .link()
                                                            .small()
                                                            .mb_2()
                                                            .text_color(cx.theme().foreground)
                                                            .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                                            .when(is_present, |this| {
                                                                this.on_click(cx.listener({
                                                                    move |this, _, window, cx| {
                                                                        this.tabs.insert(node_id.clone());
                                                                        this.set_selected_item(Some(node_id.clone()), window, cx);
                                                                        cx.notify();
                                                                    }
                                                                }))
                                                            }),
                                                    )
                                                    .when_some(probability, |this, probability| {
                                                        this.child(t_v(
                                                            "item-random-box-probability-percent",
                                                            vec![("value", format!("{:.2}", probability))],
                                                        ))
                                                    }),
                                            )
                                    }
                                })),
                        )
                    }
                },
            )
            .when_some(preview_builder.optional.package_contents, {
                move |this, contents| {
                    this.child(
                        v_flex()
                            .gap_1()
                            .child(div().mt_2().text_color(cx.theme().success).child(t("item-package-contents")))
                            .children(contents.into_iter().map({
                                move |(index, item)| {
                                    let (is_present, count, node_id, grade, icon, name) = {
                                        (
                                            item.node.item.is_some(),
                                            item.count,
                                            item.node.id.clone(),
                                            item.node.item.as_ref().and_then(|f| f.upgrade()).map(|m| m.borrow().get_grade()),
                                            item.node.item.as_ref().and_then(|f| f.upgrade()).and_then(|m| m.borrow().get_icon()),
                                            item.node
                                                .item
                                                .as_ref()
                                                .and_then(|f| f.upgrade())
                                                .map(|m| m.borrow().get_localized_name())
                                                .unwrap_or_else(|| item.node.id.clone()),
                                        )
                                    };

                                    h_flex()
                                        .gap_2()
                                        .child(
                                            h_flex()
                                                .relative()
                                                .id(format!("icon-additional-{index}"))
                                                .when_none(&icon, |this| {
                                                    this.child(
                                                        div()
                                                            .size(px(40.))
                                                            .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color))
                                                            .border_2(),
                                                    )
                                                })
                                                .when_some(icon, |this, icon| {
                                                    this.child(
                                                        img(ImageSource::Image(icon))
                                                            .object_fit(ObjectFit::Cover)
                                                            .size(px(40.))
                                                            .border_2()
                                                            .when_some(grade.and_then(|g| g.color()), |this, color| this.border_color(color)),
                                                    )
                                                }),
                                        )
                                        .child(
                                            Button::new(format!("button-{}-{}", index, node_id))
                                                .label(format!("x{count} {}", name))
                                                .link()
                                                .small()
                                                .mb_2()
                                                .text_color(cx.theme().foreground)
                                                .when_some(grade.and_then(|g| g.color()), |this, color| this.text_color(color))
                                                .when(is_present, |this| {
                                                    this.on_click(cx.listener({
                                                        move |this, _, window, cx| {
                                                            this.tabs.insert(node_id.clone());
                                                            this.set_selected_item(Some(node_id.clone()), window, cx);
                                                            cx.notify();
                                                        }
                                                    }))
                                                }),
                                        )
                                }
                            })),
                    )
                }
            })
            .when(!preview_builder.common.fishing.is_empty(), |this| {
                this.child(div().mt_2().text_color(cx.theme().success).child(t("item-fishing-drop")))
                    .children(preview_builder.common.fishing.iter().map(|fishing_drop| {
                        let color = fishing_drop.grade.color();
                        v_flex().items_start().gap_1().child(
                            h_flex()
                                .gap_1()
                                .when_some(color, |this, color| this.text_color(color))
                                .child(fishing_drop.get_localized_fishing_map())
                                .child("-")
                                .child(fishing_drop.grade.locale_fishing())
                                .child("-")
                                .child(format!(
                                    "{}%",
                                    Decimal::from_f32(fishing_drop.probability).unwrap().round_dp(5).normalize()
                                )),
                        )
                    }))
            })
            .when(!preview_builder.common.linked_recipes.is_empty(), move |this| {
                this.child(
                    v_flex().child(
                        Button::new("button-linked-recipes")
                            .my_2()
                            .small()
                            .font_weight(FontWeight::BOLD)
                            .on_click(cx.listener({
                                let id = preview_builder.common.id.clone();
                                move |this, _, _, cx| {
                                    this.preview.entry(id.clone()).and_modify(|v| {
                                        v.linked_recipes_expanded = !v.linked_recipes_expanded;
                                    });
                                    cx.notify();
                                }
                            }))
                            .text()
                            .icon(if preview.linked_recipes_expanded {
                                IconName::Minus
                            } else {
                                IconName::Plus
                            })
                            .label(t("item-linked-recipes")),
                    ),
                )
                .when(preview.linked_recipes_expanded, |this| {
                    this.children(preview_builder.common.linked_recipes.iter().filter_map(|id| items.get(id)).map(|item| {
                        let id = item.borrow().get_id();
                        let grade = item.borrow().get_grade();

                        v_flex().items_start().child(
                            Button::new(format!("button-recipe-{}", id))
                                .label(item.borrow().get_localized_name())
                                .link()
                                .small()
                                .mb_2()
                                .when_some(grade.color(), |this, color| this.text_color(color))
                                .on_click(cx.listener({
                                    let id = id.clone();
                                    move |this, _, window, cx| {
                                        this.tabs.insert(id.clone());
                                        this.set_selected_item(Some(id.clone()), window, cx);
                                        cx.notify();
                                    }
                                })),
                        )
                    }))
                })
            })
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
            cx.write_to_clipboard(ClipboardItem::new_string(item.borrow().get_localized_name().to_string()));
            window.push_notification((NotificationType::Info, t("message-copy-item-name")), cx);
        }
    }

    pub fn set_selected_item(&mut self, item: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = item.as_ref() else {
            self.selected_item = None;
            cx.notify();
            return;
        };
        if let Some(index) = self.tabs.get_index_of(item) {
            self.tabs_scroll_handle.scroll_to_item(index);
        }
        if let Some(item) = self.game_data.items.get(item) {
            self.selected_item = Some(item.borrow().get_id());
            self.debug_preview.update(cx, |state, cx| {
                state.set_value(item.borrow().get_debug().unwrap_or_default(), window, cx);
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
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GameDataLoadingStatus {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Label::new(self.localize())
                .text_sm()
                .line_height(rems(1.8))
                .text_color(cx.theme().muted_foreground),
        )
    }
}

impl Render for GameDataView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle(cx);
        let language = LanguageController::get_current_language();
        let game_path = self.game_path.clone();

        v_flex()
            .size_full()
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_2()
                        .child("PICARUS")
                        .when_some(option_env!("VERGEN_GIT_DESCRIBE"), |this, git_describe| {
                            this.child(
                                Button::new("button-github")
                                    .occlude()
                                    .icon(IconName::Github)
                                    .text_color(cx.theme().foreground)
                                    .link()
                                    .xsmall()
                                    .label(git_describe)
                                    .on_click(|_, _, cx| cx.open_url("https://github.com/VladTheJunior/picarus")),
                            )
                        })
                        .when(!self.game_data.elapsed.is_zero(), |this| {
                            this.child(
                                Label::new(t_v(
                                    "game-data-elapsed",
                                    vec![
                                        ("elapsed", format!("{:?}", self.game_data.elapsed)),
                                        ("items", format!("{}", self.game_data.items.len())),
                                    ],
                                ))
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                            )
                        }),
                ),
            )
            .when_else(
                self.is_reading,
                |this| {
                    this.child(
                        v_flex().size_full().justify_center().items_center().child(
                            h_flex()
                                .w(px(350.))
                                .items_center()
                                .gap_5()
                                .child(
                                    ProgressCircle::new("analysis-progress")
                                        .loading(self.is_reading)
                                        .min_size(px(80.))
                                        .size(px(80.)),
                                )
                                .child(
                                    v_flex()
                                        .child(div().font_medium().child(t("game-data-loading")))
                                        .child(self.loading_status.clone()),
                                ),
                        ),
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
                                    .child(Input::new(&game_path).readonly(true).w(px(300.)).suffix(
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
                                                            .px_1()
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
                                                                                let icon = item.borrow().get_icon();
                                                                                let grade = item.borrow().get_grade();

                                                                                h_flex()
                                                                                    .w_full()
                                                                                    .id(id.clone())
                                                                                    .on_click(cx.listener({
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
                                                                                                        .when_some(grade.color(), |this, color| {
                                                                                                            this.border_color(color)
                                                                                                        })
                                                                                                        .border_2(),
                                                                                                )
                                                                                            })
                                                                                            .when_some(icon, |this, icon| {
                                                                                                this.child(
                                                                                                    img(ImageSource::Image(icon))
                                                                                                        .object_fit(ObjectFit::Cover)
                                                                                                        .size(px(40.))
                                                                                                        .border_2()
                                                                                                        .when_some(grade.color(), |this, color| {
                                                                                                            this.border_color(color)
                                                                                                        }),
                                                                                                )
                                                                                            }),
                                                                                    )
                                                                                    .child(
                                                                                        div()
                                                                                            .truncate()
                                                                                            .w_full()
                                                                                            .text_sm()
                                                                                            .child(item.borrow().get_localized_name())
                                                                                            .when_some(grade.color(), |this, color| {
                                                                                                this.text_color(color)
                                                                                            }),
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
                                                TabBar::new("tabs")
                                                    .scrollbar_width(px(0.0))
                                                    .min_w_0()
                                                    .w_full()
                                                    .track_scroll(&self.tabs_scroll_handle)
                                                    .when_some(self.selected_item.as_ref().and_then(|f| self.tabs.get_index_of(f)), |this, index| {
                                                        this.selected_index(index)
                                                    })
                                                    .when(!self.tabs.is_empty(), |this| {
                                                        this.prefix(
                                                            Button::new("tabs-menu")
                                                                .icon(IconName::EllipsisVertical)
                                                                .custom(ButtonCustomVariant::new(cx))
                                                                .small()
                                                                .dropdown_menu({
                                                                    let entity = cx.entity();
                                                                    move |mut menu, window, cx| {
                                                                        for item_id in &entity.read(cx).tabs {
                                                                            let item = entity.read(cx).game_data.items.get(item_id);

                                                                            menu = menu.scrollable(true).max_h(px(300.)).when_some(
                                                                                item,
                                                                                |this, item| {
                                                                                    this.item(
                                                                                        PopupMenuItem::new(item.borrow().get_localized_name())
                                                                                            .checked(
                                                                                                entity
                                                                                                    .read(cx)
                                                                                                    .selected_item
                                                                                                    .as_ref()
                                                                                                    .is_some_and(|f| f == item_id),
                                                                                            )
                                                                                            .on_click({
                                                                                                let item_id = item_id.clone();

                                                                                                window.listener_for(
                                                                                                    &entity,
                                                                                                    move |this, _, window, cx| {
                                                                                                        this.set_selected_item(
                                                                                                            Some(item_id.clone()),
                                                                                                            window,
                                                                                                            cx,
                                                                                                        );
                                                                                                        cx.notify();
                                                                                                    },
                                                                                                )
                                                                                            }),
                                                                                    )
                                                                                },
                                                                            );
                                                                        }
                                                                        menu
                                                                    }
                                                                }),
                                                        )
                                                        .children(
                                                            self.tabs.iter().filter_map(|item_id| self.game_data.items.get(item_id)).map(|item| {
                                                                let item_id = item.borrow().get_id();

                                                                Tab::new()
                                                                    .child(
                                                                        div()
                                                                            .px_1()
                                                                            .w_full()
                                                                            .child(item.borrow().get_localized_name())
                                                                            .when_some(item.borrow().get_grade().color(), |this, color| {
                                                                                this.text_color(color)
                                                                            }),
                                                                    )
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
                                                            }),
                                                        )
                                                    }),
                                            )
                                            .when_some(
                                                self.selected_item
                                                    .as_ref()
                                                    .and_then(|selected_item_id| self.game_data.items.get(selected_item_id).map(|f| f.borrow())),
                                                |this, selected_item| {
                                                    let grade = selected_item.get_grade();

                                                    let preview = self.preview.entry(selected_item.get_id()).or_insert_with(|| {
                                                        let mut p = PreviewValues::default();
                                                        if selected_item.item_type() == ItemType::Accessory
                                                            || selected_item.item_type() == ItemType::Armor
                                                            || selected_item.item_type() == ItemType::Weapon
                                                            || selected_item.item_type() == ItemType::SecondaryWeapon
                                                            || selected_item.item_type() == ItemType::Relic
                                                        {
                                                            p.quality = Some(Quality::Simple);
                                                        }
                                                        p
                                                    });
                                                    let transcendence_effect =
                                                        (preview.transcendence != 0).then(|| preview.transcendence as f32 * 0.05);

                                                    let preview_builder = match &*selected_item {
                                                        crate::game_data::items::Item::Armor(armor) => {
                                                            let random_effects = self.game_data.get_random_effects(
                                                                grade,
                                                                armor.common.item_level,
                                                                &armor.common.usable_class,
                                                                &format!("{}_{}", armor.armor_type, armor.equip_slot),
                                                            );

                                                            let quality_effect = self.game_data.get_quality_effect(
                                                                armor.get_type(),
                                                                armor.common.item_level,
                                                                preview.quality,
                                                            );

                                                            let (magic_tempering_effect, physical_tempering_effect) = self
                                                                .game_data
                                                                .get_tempering_effect(armor.get_full_type(), armor.common.item_level)
                                                                .and_then(|f| {
                                                                    preview.total_tempering.checked_sub(1).and_then(|index| {
                                                                        f.defense_ratios.get(index as usize).map(|f| {
                                                                            (
                                                                                f / 100.0 * (armor.magical_defense),
                                                                                f / 100.0
                                                                                    * (armor.physical_defense + quality_effect.unwrap_or_default()),
                                                                            )
                                                                        })
                                                                    })
                                                                })
                                                                .map_or((None, None), |(x, y)| (Some(x), Some(y)));

                                                            armor
                                                                .build_preview()
                                                                .magic_defense_tempering_effect(magic_tempering_effect)
                                                                .physic_defense_tempering_effect(physical_tempering_effect)
                                                                .quality_effect(quality_effect)
                                                                .random_effects(random_effects)
                                                                .transcendence_effect(transcendence_effect)
                                                        }

                                                        crate::game_data::items::Item::SecondaryWeapon(secondary_weapon) => {
                                                            let random_effects = self.game_data.get_random_effects(
                                                                grade,
                                                                secondary_weapon.common.item_level,
                                                                &secondary_weapon.common.usable_class,
                                                                &secondary_weapon.weapon_type,
                                                            );
                                                            let quality_effect = self.game_data.get_quality_effect(
                                                                secondary_weapon.get_type(),
                                                                secondary_weapon.common.item_level,
                                                                preview.quality,
                                                            );

                                                            let (magic_tempering_effect, physical_tempering_effect) = self
                                                                .game_data
                                                                .get_tempering_effect(
                                                                    secondary_weapon.get_full_type(),
                                                                    secondary_weapon.common.item_level,
                                                                )
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
                                                            secondary_weapon
                                                                .build_preview()
                                                                .magic_defense_tempering_effect(magic_tempering_effect)
                                                                .physic_defense_tempering_effect(physical_tempering_effect)
                                                                .quality_effect(quality_effect)
                                                                .random_effects(random_effects)
                                                                .transcendence_effect(transcendence_effect)
                                                        }

                                                        crate::game_data::items::Item::Weapon(weapon) => {
                                                            let random_effects = self.game_data.get_random_effects(
                                                                grade,
                                                                weapon.common.item_level,
                                                                &weapon.common.usable_class,
                                                                &weapon.weapon_type,
                                                            );
                                                            let quality_effect = self.game_data.get_quality_effect(
                                                                weapon.get_type(),
                                                                weapon.common.item_level,
                                                                preview.quality,
                                                            );

                                                            let tempering_effect = self
                                                                .game_data
                                                                .get_tempering_effect(weapon.get_full_type(), weapon.common.item_level)
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
                                                            weapon
                                                                .build_preview()
                                                                .attack_tempering_effect(tempering_effect)
                                                                .quality_effect(quality_effect)
                                                                .random_effects(random_effects)
                                                                .transcendence_effect(transcendence_effect)
                                                        }

                                                        crate::game_data::items::Item::Material(material) => material.build_preview(),
                                                        crate::game_data::items::Item::Recipe(recipe) => recipe.build_preview(),
                                                        crate::game_data::items::Item::Consume(consume) => consume.build_preview(),
                                                        crate::game_data::items::Item::Fellow(fellow) => fellow.build_preview(),
                                                        crate::game_data::items::Item::Elluns(elluns) => elluns.build_preview(),
                                                        crate::game_data::items::Item::FellowConsume(fellow_consume) => {
                                                            fellow_consume.build_preview()
                                                        }
                                                        crate::game_data::items::Item::SkillBook(skill_book) => skill_book.build_preview(),
                                                        crate::game_data::items::Item::SealedFellow(sealed_fellow) => sealed_fellow.build_preview(),
                                                        crate::game_data::items::Item::Boost(boost) => boost.build_preview(),
                                                        crate::game_data::items::Item::Relic(relic) => {
                                                            let quality_effect = self.game_data.get_quality_effect(
                                                                relic.get_type(),
                                                                relic.common.item_level,
                                                                preview.quality,
                                                            );

                                                            let random_effects = self.game_data.get_random_effects(
                                                                grade,
                                                                relic.common.item_level,
                                                                &relic.common.usable_class,
                                                                &relic.equip_slot,
                                                            );

                                                            relic.build_preview().random_effects(random_effects).quality_effect(quality_effect)
                                                        }
                                                        crate::game_data::items::Item::Bracelet(bracelet) => bracelet.build_preview(),
                                                        crate::game_data::items::Item::Bag(bag) => bag.build_preview(),
                                                        crate::game_data::items::Item::FellowBook(fellow_book) => fellow_book.build_preview(),
                                                        crate::game_data::items::Item::Style(style) => style.build_preview(),
                                                        crate::game_data::items::Item::FellowStyle(fellow_style) => fellow_style.build_preview(),
                                                        crate::game_data::items::Item::RandomBox(random_box) => random_box.build_preview(),
                                                        crate::game_data::items::Item::Package(package) => package.build_preview(),
                                                        crate::game_data::items::Item::Exchange(exchange) => exchange.build_preview(),
                                                        crate::game_data::items::Item::Quest(quest) => quest.build_preview(),
                                                        crate::game_data::items::Item::Gem(gem) => gem.build_preview(),
                                                        crate::game_data::items::Item::Event(event) => event.build_preview(),
                                                        crate::game_data::items::Item::FellowEquip(fellow_equip) => fellow_equip.build_preview(),
                                                        crate::game_data::items::Item::Accessory(accessory) => {
                                                            let random_effects = self.game_data.get_random_effects(
                                                                grade,
                                                                accessory.common.item_level,
                                                                &accessory.common.usable_class,
                                                                &accessory.accessory_type,
                                                            );

                                                            let quality_effect = self.game_data.get_quality_effect(
                                                                accessory.get_type(),
                                                                accessory.common.item_level,
                                                                preview.quality,
                                                            );

                                                            let tempering_effect = self
                                                                .game_data
                                                                .get_tempering_effect(accessory.get_full_type(), accessory.common.item_level)
                                                                .and_then(|f| {
                                                                    preview.total_tempering.checked_sub(1).and_then(|index| {
                                                                        f.defense_ratios
                                                                            .get(index as usize)
                                                                            .map(|f| f / 100.0 * (accessory.magic_defense))
                                                                    })
                                                                });
                                                            accessory
                                                                .build_preview()
                                                                .magic_defense_tempering_effect(tempering_effect)
                                                                .quality_effect(quality_effect)
                                                                .random_effects(random_effects)
                                                                .transcendence_effect(transcendence_effect)
                                                        }
                                                    };

                                                    this.child(
                                                        div()
                                                            .flex()
                                                            .size_full()
                                                            .relative()
                                                            .when_else(
                                                                self.debug,
                                                                |this| {
                                                                    this.child(
                                                                        Editor::new(&self.debug_preview)
                                                                            .font_family(cx.theme().mono_font_family.clone())
                                                                            .text_size(cx.theme().mono_font_size)
                                                                            .readonly(true)
                                                                            .bordered(false)
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
                                                                                    selected_item.item_type(),
                                                                                    preview_builder,
                                                                                    preview,
                                                                                    &self.game_data.items,
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
                            .appearance(false)
                            .flex_shrink_0()
                            .w(px(284.))
                            .disabled(self.is_reading)
                            .prefix(Icon::new(IconName::Search).small())
                            .small(),
                    )
                    .left(Separator::vertical())
                    .left({
                        let entity = self.filters.item_type_state.clone();
                        let status = entity.read_with(cx, |this, _| {
                            let selected_len = this.selected_values().len();
                            if selected_len == 0 {
                                return Some(false);
                            }
                            if selected_len == ItemType::iter().len() {
                                return Some(true);
                            }
                            None
                        });
                        Combobox::new(&self.filters.item_type_state)
                            .appearance(false)
                            .w(px(200.))
                            .flex_shrink_0()
                            .disabled(self.is_reading)
                            .small()
                            .render_trigger(move |_, _, _| div().child(t("item-types")))
                            .footer(move |_, _| {
                                let entity = entity.clone();
                                Button::new("check-selection-grade")
                                    .ghost()
                                    .map(move |this| match status {
                                        Some(true) => this.icon(Icon::new(AppIcon::SquareCheckBig)).on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.clear_selection(cx);
                                                cx.notify();
                                            });
                                        }),
                                        Some(false) => this.icon(Icon::new(AppIcon::Square)).on_click(move |_, window, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.set_selected_indices(ItemType::iter().enumerate().map(|(i, _)| IndexPath::new(i)), window, cx);
                                                cx.emit(ComboboxEvent::Change(this.selected_values()));
                                                cx.notify();
                                            });
                                        }),
                                        None => this.icon(Icon::new(AppIcon::SquareMinus)).on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.clear_selection(cx);
                                                cx.notify();
                                            });
                                        }),
                                    })
                                    .label(t("checkbox-check-all"))
                                    .small()
                                    .w_full()
                                    .justify_start()
                                    .into_any_element()
                            })
                    })
                    .left(Separator::vertical())
                    .left({
                        let entity = self.filters.grade_state.clone();
                        let status = entity.read_with(cx, |this, _| {
                            let selected_len = this.selected_values().len();
                            if selected_len == 0 {
                                return Some(false);
                            }
                            if selected_len == Grade::iter().len() {
                                return Some(true);
                            }
                            None
                        });
                        Combobox::new(&self.filters.grade_state)
                            .appearance(false)
                            .w(px(150.))
                            .flex_shrink_0()
                            .disabled(self.is_reading)
                            .small()
                            .render_trigger(move |_, _, _| div().child(t("item-grades")))
                            .footer(move |_, _| {
                                let entity = entity.clone();
                                Button::new("check-selection-grade")
                                    .ghost()
                                    .map(move |this| match status {
                                        Some(true) => this.icon(Icon::new(AppIcon::SquareCheckBig)).on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.clear_selection(cx);
                                                cx.notify();
                                            });
                                        }),
                                        Some(false) => this.icon(Icon::new(AppIcon::Square)).on_click(move |_, window, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.set_selected_indices(Grade::iter().enumerate().map(|(i, _)| IndexPath::new(i)), window, cx);
                                                cx.emit(ComboboxEvent::Change(this.selected_values()));
                                                cx.notify();
                                            });
                                        }),
                                        None => this.icon(Icon::new(AppIcon::SquareMinus)).on_click(move |_, _, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.clear_selection(cx);
                                                cx.notify();
                                            });
                                        }),
                                    })
                                    .label(t("checkbox-check-all"))
                                    .small()
                                    .w_full()
                                    .justify_start()
                                    .into_any_element()
                            })
                    })
                    .left(Separator::vertical())
                    .left(
                        Combobox::new(&self.filters.effects_state)
                            .appearance(false)
                            .w(px(300.))
                            .flex_shrink_0()
                            .disabled(self.is_reading)
                            .small()
                            .cleanable(true)
                            .placeholder(t("item-effects")),
                    )
                    .left(Separator::vertical())
                    .left(
                        Button::new("filter-additional")
                            .ghost()
                            .xsmall()
                            .disabled(self.is_reading)
                            .icon(AppIcon::EllipsisVertical)
                            .dropdown_menu({
                                let entity = cx.entity();
                                move |mut menu, window, cx| {
                                    menu = menu.submenu(t("filter-fishing"), window, cx, {
                                        let entity = entity.clone();
                                        move |mut menu, window, cx| {
                                            let fishing = entity
                                                .read(cx)
                                                .game_data
                                                .fishing
                                                .iter()
                                                .map(|f| (f.area.clone(), f.get_localized_fishing_map()))
                                                .collect::<IndexMap<_, _>>();

                                            for (area_id, area) in fishing {
                                                let is_checked = entity
                                                    .read(cx)
                                                    .filters
                                                    .additional_filter
                                                    .as_ref()
                                                    .is_some_and(|x| *x == AdditionalFilter::Fishing(area_id.clone()));

                                                menu = menu.item(PopupMenuItem::new(area).checked(is_checked).on_click({
                                                    window.listener_for(&entity, move |this, _, _, cx| {
                                                        if is_checked {
                                                            this.filters.additional_filter = None;
                                                        } else {
                                                            this.filters.additional_filter = Some(AdditionalFilter::Fishing(area_id.clone()));
                                                        }
                                                        this.apply_filter_and_resort();
                                                        cx.notify();
                                                    })
                                                }))
                                            }
                                            menu
                                        }
                                    });

                                    let is_evolution_checked = entity
                                        .read(cx)
                                        .filters
                                        .additional_filter
                                        .as_ref()
                                        .is_some_and(|x| *x == AdditionalFilter::Evolution);
                                    let is_synthesis_checked = entity
                                        .read(cx)
                                        .filters
                                        .additional_filter
                                        .as_ref()
                                        .is_some_and(|x| *x == AdditionalFilter::Synthesis);
                                    menu.item(PopupMenuItem::new(t("filter-evolution")).checked(is_evolution_checked).on_click({
                                        window.listener_for(&entity, move |this, _, _, cx| {
                                            if is_evolution_checked {
                                                this.filters.additional_filter = None;
                                            } else {
                                                this.filters.additional_filter = Some(AdditionalFilter::Evolution);
                                            }
                                            this.apply_filter_and_resort();
                                            cx.notify();
                                        })
                                    }))
                                    .item(
                                        PopupMenuItem::new(t("filter-synthesis")).checked(is_synthesis_checked).on_click({
                                            window.listener_for(&entity, move |this, _, _, cx| {
                                                if is_synthesis_checked {
                                                    this.filters.additional_filter = None;
                                                } else {
                                                    this.filters.additional_filter = Some(AdditionalFilter::Synthesis);
                                                }
                                                this.apply_filter_and_resort();
                                                cx.notify();
                                            })
                                        }),
                                    )
                                }
                            }),
                    )
                    .right(
                        Button::new("button-export")
                            .ghost()
                            .xsmall()
                            .icon(AppIcon::Upload)
                            .loading(self.is_exporting)
                            .cursor_pointer()
                            .disabled(self.is_reading)
                            .label(t("button-export"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.export_xlsx(window, cx);
                            })),
                    )
                    .right(Separator::vertical())
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
