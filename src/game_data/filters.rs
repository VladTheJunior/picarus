use std::collections::HashSet;

use crate::{
    game_data::{DataType, Grade, ItemType},
    game_data_view::GameDataView,
    language::t_v,
};
use gpui::{AppContext, Context, Entity, SharedString, Window};
use gpui_component::{
    IndexPath,
    combobox::ComboboxState,
    input::{InputEvent, InputState},
    select::SearchableVec,
};
use gpui_component::{combobox::*, searchable_list::SearchableListItem};
use strum::IntoEnumIterator;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ItemEffectFilter {
    pub key: SharedString,
}

impl ItemEffectFilter {
    pub fn locale(&self) -> SharedString {
        SharedString::new(t_v(&self.key, vec![("value", "")]).trim_end_matches(&['-']).trim_end())
    }
}

pub struct GameDataFilters {
    pub search_state: Entity<InputState>,
    pub item_type_state: Entity<ComboboxState<SearchableVec<ItemType>>>,
    pub grade_state: Entity<ComboboxState<SearchableVec<Grade>>>,
    pub effects_state: Entity<ComboboxState<SearchableVec<ItemEffectFilter>>>,
    pub input: String,
    pub item_type: HashSet<ItemType>,
    pub grade: HashSet<Grade>,
    pub effects: Option<SharedString>,
}

impl SearchableListItem for ItemEffectFilter {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.locale()
    }

    fn value(&self) -> &SharedString {
        &self.key
    }
}

impl SearchableListItem for ItemType {
    type Value = ItemType;

    fn title(&self) -> SharedString {
        self.locale()
    }

    fn value(&self) -> &ItemType {
        &self
    }

    fn matches(&self, _query: &str) -> bool {
        true
    }
}

impl SearchableListItem for Grade {
    type Value = Grade;

    fn title(&self) -> SharedString {
        self.locale()
    }

    fn value(&self) -> &Grade {
        &self
    }

    fn matches(&self, _query: &str) -> bool {
        true
    }
}

impl GameDataFilters {
    pub fn new(window: &mut Window, cx: &mut Context<GameDataView>) -> Self {
        let search_state = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        cx.subscribe(&search_state, move |this, state, event, cx| match event {
            InputEvent::Change => {
                this.filters.input = state.read(cx).value().to_lowercase();
                this.apply_filter_and_resort();
                cx.notify();
            }
            _ => {}
        })
        .detach();

        let item_type_state = cx.new(|cx| {
            ComboboxState::new(
                SearchableVec::new(ItemType::iter().collect::<Vec<_>>()),
                ItemType::iter().enumerate().map(|(i, _)| IndexPath::new(i)).collect(),
                window,
                cx,
            )
            .multiple(true)
            .searchable(false)
        });
        cx.subscribe(&item_type_state, |view, _, event, cx| match event {
            ComboboxEvent::Change(values) => {
                view.filters.item_type = values.iter().cloned().collect();
                view.apply_filter_and_resort();
                cx.notify();
            }
            ComboboxEvent::Confirm(_) => {}
        })
        .detach();

        let grade_state = cx.new(|cx| {
            ComboboxState::new(
                SearchableVec::new(Grade::iter().collect::<Vec<_>>()),
                Grade::iter().enumerate().map(|(i, _)| IndexPath::new(i)).collect(),
                window,
                cx,
            )
            .multiple(true)
            .searchable(false)
        });
        cx.subscribe(&grade_state, |view, _, event, cx| match event {
            ComboboxEvent::Change(values) => {
                view.filters.grade = values.iter().cloned().collect();
                view.apply_filter_and_resort();
                cx.notify();
            }
            ComboboxEvent::Confirm(_) => {}
        })
        .detach();

        let effects_state = cx.new(|cx| {
            ComboboxState::new(SearchableVec::new(vec![]), vec![], window, cx)
                .multiple(false)
                .searchable(true)
        });
        cx.subscribe(&effects_state, |view, _, event, cx| match event {
            ComboboxEvent::Change(values) => {
                view.filters.effects = values.iter().next().cloned();
                view.apply_filter_and_resort();
                cx.notify();
            }
            ComboboxEvent::Confirm(_) => {}
        })
        .detach();

        Self {
            search_state,
            item_type_state,
            input: String::default(),
            item_type: ItemType::iter().collect(),
            grade: Grade::iter().collect(),
            grade_state,
            effects_state,
            effects: None,
        }
    }

    pub fn check_item(&self, item: &DataType) -> bool {
        item.matches(&self.input, &self.item_type, &self.grade, &self.effects)
    }
}
