pub mod accessory;
pub mod armor;
pub mod bag;
pub mod boost;
pub mod consume;
pub mod exchange;
pub mod fellow_equip;
pub mod gem;
pub mod material;
pub mod package;
pub mod weapon;

pub mod bracelet;
pub mod elluns;
pub mod event;
pub mod fellow;
pub mod fellow_book;
pub mod fellow_consume;
pub mod fellow_style;
pub mod quest;
pub mod random_box;
pub mod recipe;
pub mod relic;
pub mod sealed_fellow;
pub mod secondary_weapon;
pub mod skill_book;
pub mod style;
pub mod monster;

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io::{Cursor, Read, SeekFrom},
    rc::Weak,
    sync::Arc,
};

use crate::{
    game_data::{
        AsyncBufReadExtReadString, common::Common, evolution::Evolution, fellow_combination::FellowCombination, filters::AdditionalFilter, fishing::Fishing, grade::Grade, items::{
            accessory::Accessory, armor::Armor, bag::Bag, boost::Boost, bracelet::Bracelet, consume::Consume, elluns::Elluns, event::Event, exchange::Exchange, fellow::Fellow, fellow_book::FellowBook, fellow_consume::FellowConsume, fellow_equip::FellowEquip, fellow_style::FellowStyle, gem::Gem, material::Material, monster::Monster, package::Package, quest::Quest, random_box::RandomBox, recipe::Recipe, relic::Relic, sealed_fellow::SealedFellow, secondary_weapon::SecondaryWeapon, skill_book::SkillBook, style::Style, weapon::Weapon,
        }, locale::Locale, synthesis_fellows::SynthesisFellows, synthesis_parts::SynthesisParts,
    }, language::t,
};
use crate::{
    game_data::{DataFormat, DebugValue, TagType, effects::EffectKind, read_definitions, read_item_count, read_offsets},
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use enum_dispatch::enum_dispatch;
use gpui_kit::{Image, SharedString};
use indexmap::IndexMap;
use strum::EnumIter;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt, BufReader};
use zip::ZipArchive;
#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ArmorClassKind {
    Magic(ArmorTypes),
    Physical(ArmorTypes),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ArmorTypes {
    Helmet,
    Pauldron,
    Armor,
    Gloves,
    Boots,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ItemSubType {
    Necklage,
    Ring,
    Armor(ArmorClassKind),
    Dagger,
    Sword,
    Greatsword,
    Scepter,
    Staff,
    Lance,
    Crossbow,
    Bow,
    Wand,
    Shield,
    Crest,
    Vambrace,
    TeddyBear,
    Relic,
}

impl TryFrom<&str> for ItemSubType {
    type Error = String;

    fn try_from(other: &str) -> Result<Self, Self::Error> {
        match other {
            "ne" => Ok(Self::Necklage),
            "ri" => Ok(Self::Ring),
            "pl_ha" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Helmet))),
            "pl_sh" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Pauldron))),
            "pl_ja" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Armor))),
            "pl_gl" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Gloves))),
            "pl_bo" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Boots))),

            "le_ha" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Helmet))),
            "le_sh" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Pauldron))),
            "le_ja" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Armor))),
            "le_gl" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Gloves))),
            "le_bo" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Boots))),

            "ch_ha" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Helmet))),
            "ch_sh" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Pauldron))),
            "ch_ja" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Armor))),
            "ch_gl" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Gloves))),
            "ch_bo" => Ok(Self::Armor(ArmorClassKind::Physical(ArmorTypes::Boots))),

            "cl_ha" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Helmet))),
            "cl_sh" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Pauldron))),
            "cl_ja" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Armor))),
            "cl_gl" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Gloves))),
            "cl_bo" => Ok(Self::Armor(ArmorClassKind::Magic(ArmorTypes::Boots))),

            "d1" => Ok(Self::Dagger),
            "s1" => Ok(Self::Sword),
            "s2" => Ok(Self::Greatsword),
            "m1" => Ok(Self::Scepter),
            "m2" => Ok(Self::Staff),
            "l2" => Ok(Self::Lance),
            "c2" => Ok(Self::Crossbow),
            "b1" => Ok(Self::Bow),
            "w1" => Ok(Self::Wand),
            "sd" => Ok(Self::Shield),
            "at" => Ok(Self::Crest),
            "ga" => Ok(Self::Vambrace),
            "tb" => Ok(Self::TeddyBear),
            "re" => Ok(Self::Relic),
            unk => Err(format!("Cannot convert {} item subtype", unk)),
        }
    }
}

#[derive(EnumIter, Eq, PartialEq, Hash, Clone, Copy)]
pub enum ItemType {
    Armor,
    SecondaryWeapon,
    Weapon,
    Accessory,
    Material,
    Recipe,
    FellowEquip,
    Consume,
    Boost,
    Gem,
    SealedFellow,
    SkillBook,
    Exchange,
    RandomBox,
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
    Monster
}

impl ItemType {
    pub fn locale(&self) -> SharedString {
        match self {
            ItemType::Armor => t("item-type-armor"),
            ItemType::SecondaryWeapon => t("item-type-secondary-weapon"),
            ItemType::Weapon => t("item-type-weapon"),
            ItemType::Accessory => t("item-type-accessory"),
            ItemType::Material => t("item-type-material"),
            ItemType::Recipe => t("item-type-recipe"),
            ItemType::FellowEquip => t("item-type-fellow-equip"),
            ItemType::Consume => t("item-type-consume"),
            ItemType::Boost => t("item-type-boost"),
            ItemType::Gem => t("item-type-gem"),
            ItemType::SealedFellow => t("item-type-sealed-fellow"),
            ItemType::SkillBook => t("item-type-skill-book"),
            ItemType::Exchange => t("item-type-exchange"),
            ItemType::RandomBox => t("item-type-random-box"),
            ItemType::Package => t("item-type-package"),
            ItemType::Style => t("item-type-style"),
            ItemType::Bag => t("item-type-bag"),
            ItemType::FellowStyle => t("item-type-fellow-style"),
            ItemType::FellowConsume => t("item-type-fellow-consume"),
            ItemType::Quest => t("item-type-quest"),
            ItemType::Bracelet => t("item-type-bracelet"),
            ItemType::Relic => t("item-type-relic"),
            ItemType::FellowBook => t("item-type-fellow-book"),
            ItemType::Event => t("item-type-event"),
            ItemType::Elluns => t("item-type-elluns"),
            ItemType::Fellow => t("item-type-fellow"),
            ItemType::Monster => t("item-type-monster"),
        }
    }
}

#[enum_dispatch]
pub trait ItemTrait {
    fn common(&self) -> &Common;
    fn common_mut(&mut self) -> &mut Common;

    fn debug(&self) -> &[u8];

    fn get_debug(&self) -> Result<String> {
        Ok(String::from_utf8(lz4_flex::block::decompress_size_prepended(self.debug())?)?)
    }

    fn get_id(&self) -> SharedString {
        self.common().id.clone()
    }
    fn get_locale(&self) -> Option<&Locale> {
        self.common().locale.as_ref()
    }

    fn get_localized_name(&self) -> SharedString {
        self.common().get_localized_name()
    }

    fn get_icon(&self) -> Option<Arc<Image>> {
        self.common().icon.clone()
    }
    fn get_grade(&self) -> Grade {
        self.common().grade
    }

    fn get_unique_effects(&self) -> Vec<EffectKind> {
        self.common().get_unique_effects()
    }

    fn get_type(&self) -> Option<SharedString> {
        None
    }

    fn get_full_type(&self) -> Option<SharedString> {
        None
    }

    fn set_fishing_drop(&mut self, fishing: &Vec<Fishing>) {
        self.common_mut().set_fishing_drop(fishing);
    }

    fn set_fellow_combinations(&mut self, fellow_combinations: &Vec<FellowCombination>) {
        self.common_mut().set_fellow_combinations(fellow_combinations);
    }

    fn set_evolution(&mut self, evolution: &Vec<Evolution>) {
        self.common_mut().set_evolution(evolution);
    }

    fn set_synthesis_parts(&mut self, synthesis_parts: &Vec<SynthesisParts>) {
        self.common_mut().set_synthesis_parts(synthesis_parts);
    }

    fn set_synthesis_fellows(&mut self, synthesis_fellows: &Vec<SynthesisFellows>) {
        self.common_mut().set_synthesis_fellows(synthesis_fellows);
    }

    fn build_preview(&self) -> PreviewBuilder<'_>;
}

#[derive(Default, Clone)]
pub struct ItemNode {
    pub id: SharedString,
    pub item: Option<Weak<RefCell<Item>>>,
}

#[enum_dispatch(ItemTrait)]
#[derive(Clone)]
pub enum Item {
    SecondaryWeapon(SecondaryWeapon),
    Weapon(Weapon),
    Armor(Armor),
    Accessory(Accessory),
    Material(Material),
    Recipe(Recipe),
    FellowEquip(FellowEquip),
    Consume(Consume),
    Boost(Boost),
    Gem(Gem),
    SealedFellow(SealedFellow),
    SkillBook(SkillBook),
    Exchange(Exchange),
    RandomBox(RandomBox),
    Package(Package),
    Style(Style),
    Bag(Bag),
    FellowStyle(FellowStyle),
    FellowConsume(FellowConsume),
    Quest(Quest),
    Bracelet(Bracelet),
    Relic(Relic),
    FellowBook(FellowBook),
    Event(Event),
    Elluns(Elluns),
    Fellow(Fellow),
    Monster(Monster),
}

impl Item {
    pub fn item_type(&self) -> ItemType {
        match self {
            Self::Weapon(_) => ItemType::Weapon,
            Self::Armor(_) => ItemType::Armor,
            Self::Accessory(_) => ItemType::Accessory,
            Self::SecondaryWeapon(_) => ItemType::SecondaryWeapon,
            Self::Material(_) => ItemType::Material,
            Self::Recipe(_) => ItemType::Recipe,
            Self::FellowEquip(_) => ItemType::FellowEquip,
            Self::Consume(_) => ItemType::Consume,
            Self::Boost(_) => ItemType::Boost,
            Self::Gem(_) => ItemType::Gem,
            Self::SealedFellow(_) => ItemType::SealedFellow,
            Self::SkillBook(_) => ItemType::SkillBook,
            Self::Exchange(_) => ItemType::Exchange,
            Self::RandomBox(_) => ItemType::RandomBox,
            Self::Package(_) => ItemType::Package,
            Self::Style(_) => ItemType::Style,
            Self::Bag(_) => ItemType::Bag,
            Self::FellowStyle(_) => ItemType::FellowStyle,
            Self::FellowConsume(_) => ItemType::FellowConsume,
            Self::Quest(_) => ItemType::Quest,
            Self::Bracelet(_) => ItemType::Bracelet,
            Self::Relic(_) => ItemType::Relic,
            Self::FellowBook(_) => ItemType::FellowBook,
            Self::Event(_) => ItemType::Event,
            Self::Elluns(_) => ItemType::Elluns,
            Self::Fellow(_) => ItemType::Fellow,
            Self::Monster(_) => ItemType::Monster,
        }
    }

    pub fn filter_fishing(&self, filter: &SharedString) -> bool {
        return self.common().fishing.iter().any(|f| f.area == *filter);
    }

    pub fn filter_evolution(&self) -> bool {
        return !self.common().evolution.is_empty();
    }

    pub fn filter_synthesis(&self) -> bool {
        return !self.common().synthesis_fellows.is_empty() || !self.common().synthesis_parts.is_empty();
    }

    pub fn filter_fellow_combination(&self) -> bool {
        return !self.common().fellow_combinations.is_empty();
    }

    pub fn filter_effect(&self, filter: &Option<SharedString>) -> bool {
        if let Some(filter) = filter {
            let effects = self
                .get_unique_effects()
                .into_iter()
                .filter_map(|e| match e {
                    super::effects::EffectKind::Common { id: _, effect } => effect.parsed.as_ref().map(|(key, _)| key.clone()),
                    super::effects::EffectKind::MinMaxNoStep { id: _, effect } => effect.parsed.as_ref().map(|(key, _, _)| key.clone()),
                    super::effects::EffectKind::MinMaxStep { id: _, effect } => effect.parsed.as_ref().map(|(key, _, _, _)| key.clone()),
                })
                .collect::<HashSet<SharedString>>();
            return effects.contains(filter);
        }
        return true;
    }

    pub fn matches(
        &self,
        input: &str,
        types: &HashSet<ItemType>,
        grades: &HashSet<Grade>,
        effect: &Option<SharedString>,
        additional_filter: &Option<AdditionalFilter>,
    ) -> bool {
        let include = types.contains(&self.item_type());

        if !include {
            return false;
        }

        if !self.filter_effect(effect) {
            return false;
        }

        match additional_filter {
            Some(AdditionalFilter::Evolution) => {
                if !self.filter_evolution() {
                    return false;
                }
            }
            Some(AdditionalFilter::Synthesis) => {
                if !self.filter_synthesis() {
                    return false;
                }
            }
            Some(AdditionalFilter::FellowCombination) => {
                if !self.filter_fellow_combination() {
                    return false;
                }
            }
            Some(AdditionalFilter::Fishing(fishing)) => {
                if !self.filter_fishing(fishing) {
                    return false;
                }
            }
            None => {}
        }

        let grade = self.get_grade();

        if !grades.contains(&grade) {
            return false;
        }

        if input.is_empty() {
            return true;
        }

        let id = self.get_id();
        let locale = self.get_locale();
        id.to_lowercase().contains(&input)
            || locale.is_some_and(|l| l.rus.to_lowercase().contains(&input))
            || locale.is_some_and(|l| l.eng.to_lowercase().contains(&input))
    }
}

pub trait ReadableItem: Sized + Default {
    const FORMAT: DataFormat;
    type Key: Eq + std::hash::Hash;
    type CollectionItem;

    fn new_collection_item(item: Self) -> Self::CollectionItem;

    fn key(&self) -> Self::Key;

    fn debug_mut(&mut self) -> &mut Vec<u8>;

    async fn read_all<R: std::io::Read + std::io::Seek>(
        gamedatas_zip: &mut ZipArchive<R>,
        data_path: &str,
    ) -> Result<HashMap<Self::Key, Self::CollectionItem>> {
        let mut file = gamedatas_zip.by_path(data_path)?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;

        let mut reader = BufReader::new(Cursor::new(data.as_slice()));

        let definitions = read_definitions(&mut reader).await?;
        //debug!(?definitions);
        let item_count = read_item_count(&mut reader).await?;
        let offsets = read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut items = HashMap::with_capacity(item_count);

        for idx in 0..item_count {
            let item = Self::default().read(&mut reader, &offsets, idx, &definitions, global_offset).await?;
            items.insert(item.key(), Self::new_collection_item(item));
        }

        Ok(items)
    }

    async fn read_all_vec<R: std::io::Read + std::io::Seek>(gamedatas_zip: &mut ZipArchive<R>, data_path: &str) -> Result<Vec<Self::CollectionItem>> {
        let mut file = gamedatas_zip.by_path(data_path)?;
        let mut data = vec![];
        file.read_to_end(&mut data)?;

        let mut reader = BufReader::new(Cursor::new(data.as_slice()));

        let definitions = read_definitions(&mut reader).await?;
        let item_count = read_item_count(&mut reader).await?;

        let offsets = read_offsets(&mut reader, item_count, definitions.len()).await?;

        let global_offset = reader.stream_position().await?;
        let mut items = Vec::with_capacity(item_count);

        for idx in 0..item_count {
            let item = Self::default().read(&mut reader, &offsets, idx, &definitions, global_offset).await?;
            items.push(Self::new_collection_item(item));
        }

        Ok(items)
    }

    async fn parse_debug<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        &mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<()> {
        let tag_count = definitions.len();
        let mut debug = IndexMap::with_capacity(tag_count);
        for (tag_idx, (key, value_type)) in definitions.iter().enumerate() {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match Self::FORMAT {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset)).await?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2)).await?;
                }
            };

            match value_type {
                TagType::String => debug.insert(key.clone(), DebugValue::String(reader.read_string(Self::FORMAT).await?)),
                TagType::Float => debug.insert(key.clone(), DebugValue::Float(reader.read_f32_le().await?)),
            };
        }
        *self.debug_mut() = lz4_flex::block::compress_prepend_size(&serde_json::to_vec_pretty(&debug)?);
        Ok(())
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self>;
}
