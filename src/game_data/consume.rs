use std::{
    cell::RefCell, collections::{BTreeSet, HashMap}, io::{Read, SeekFrom}, rc::Rc, sync::Arc,
};

use crate::{
    game_data::{
        AbstractItem, Binding, DataFormat, GameClass, Grade, Item, TagType, item_set::ItemSet, locale::Locale, product::Product, recipe::RecipeType,
    },
    language::{LanguageController, t},
};
use anyhow::Result;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::{Image, SharedString};
use tracing::warn;

#[derive(Default, Serialize)]
pub struct Consume {
    pub linked_recipes: BTreeSet<SharedString>,
    pub locale: Option<Locale>,
    pub description_locale: Option<Locale>,
    #[serde(skip)]
    pub icon: Option<Arc<Image>>,

    pub id: SharedString,
    pub name: SharedString,
    pub development_cost: f32,
    pub consumption_type: SharedString,
    pub usable_class: BTreeSet<GameClass>,
    pub usable_gender: SharedString,
    pub toggle_share: f32,
    pub grade: Option<Grade>,
    pub grade_plus: f32,
    pub required_level: u8,
    pub limit_level: f32,
    pub item_level: f32,
    pub usage_effect: SharedString,
    pub min_random_enhancement_level: f32,
    pub max_random_enhancement_level: f32,
    pub random_enhancement_probability: SharedString,
    pub enhancement_penalty_prevention_condition: SharedString,
    pub enhancement_fixed_probability: f32,
    pub enhancement_additional_probability: f32,
    pub min_enhancement_possible_level: f32,
    pub max_enhancement_possible_level: f32,
    pub enhancement_target: SharedString,
    pub extractable: f32,
    pub taming_correction: f32,
    pub usable_monster_level: f32,
    pub attribute_type: SharedString,
    pub creature_grade: SharedString,
    pub color: SharedString,
    pub durability: f32,
    pub cooldown_share: f32,
    pub cooldown: f32,
    pub same_type_cooldown_no_share: f32,
    pub purchase_price: f32,
    pub disposal_price: f32,
    pub stack_count: f32,
    pub creation_count: f32,
    pub scenario: SharedString,
    pub area_type: SharedString,
    pub usage_area: SharedString,
    pub magic_stone_skill: SharedString,
    pub magic_stone_skill_level: f32,
    pub magic_stone_equip_effect_1: SharedString,
    pub magic_stone_equip_effect_2: SharedString,
    pub magic_stone_equip_effect_3: SharedString,
    pub magic_stone_equip_effect_4: SharedString,
    pub magic_stone_equip_effect_5: SharedString,
    pub skill_effect: Option<SharedString>,
    pub no_drop: f32,
    pub untradeable: bool,
    pub unsellable: bool,
    pub indestructible: bool,
    pub true_keeping: f32,
    pub drop_level_check: SharedString,
    pub binding: Option<Binding>,
    pub binding_target: SharedString,
    pub binding_release_count: f32,
    pub consumption_limit: f32,
    pub usage_restriction: SharedString,
    pub sales_agent_category: SharedString,
    pub creation_item: SharedString,
    pub warp_map: SharedString,
    pub warp_coordinates: SharedString,
    pub combination_result: SharedString,
    pub disappearance_time: f32,
    pub seed_type: SharedString,
    pub required_farm_level: f32,
    pub crop_object: SharedString,
    pub creation_fellow: SharedString,
    pub usage_period: f32,
    pub cash: f32,
    pub fellow_experience: f32,
    pub monster_summon: SharedString,
    pub summon_time: f32,
    pub drop_level_check_ignore: f32,
    pub fellow_base_grade: SharedString,
    pub gd_grade: SharedString,
    pub raid_check: f32,
    pub contents_level: f32,
    pub engrave: f32,
    pub character_level: f32,
    pub purchase_limit: f32,
    pub unified_channel_disabled: f32,
    pub enhancement_possible_grade: SharedString,
    pub enhancement_attempt_min_level: f32,
    pub enhancement_attempt_max_level: f32,
    pub disassembly_id: SharedString,
    pub pvp_channel_usage: SharedString,
    pub unusable_area: SharedString,
    pub currency_settings_id: SharedString,
}

impl AbstractItem for Consume {
    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<String, TagType>,
        global_offset: u64,
        format: DataFormat,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        for tag_idx in 0..tag_count {
            let global_idx = item_idx * tag_count + tag_idx;
            let offset = offsets[global_idx] as u64;
            match format {
                DataFormat::String => {
                    reader.seek(SeekFrom::Start(global_offset + offset)).await?;
                }
                DataFormat::WideString => {
                    reader.seek(SeekFrom::Start(global_offset + offset * 2)).await?;
                }
            };

            match tag_idx {
                0 => self.id = SharedString::new(Self::read_string(format, reader).await?.to_uppercase()),
                1 => self.name = Self::read_string(format, reader).await?,
                2 => self.development_cost = reader.read_f32_le().await?,
                3 => self.consumption_type = Self::read_string(format, reader).await?,
                4 => {
                    let value = Self::read_string(format, reader).await?;

                    self.usable_class = value.split("_").filter_map(|c| GameClass::try_from(c).ok()).collect();
                }
                5 => self.usable_gender = Self::read_string(format, reader).await?,
                6 => self.toggle_share = reader.read_f32_le().await?,
                7 => self.grade = Grade::from_repr(reader.read_f32_le().await? as u8),
                8 => self.grade_plus = reader.read_f32_le().await?,
                9 => self.required_level = reader.read_f32_le().await? as u8,
                10 => self.limit_level = reader.read_f32_le().await?,
                11 => self.item_level = reader.read_f32_le().await?,
                12 => self.usage_effect = Self::read_string(format, reader).await?,
                13 => self.min_random_enhancement_level = reader.read_f32_le().await?,
                14 => self.max_random_enhancement_level = reader.read_f32_le().await?,
                15 => self.random_enhancement_probability = Self::read_string(format, reader).await?,
                16 => self.enhancement_penalty_prevention_condition = Self::read_string(format, reader).await?,
                17 => self.enhancement_fixed_probability = reader.read_f32_le().await?,
                18 => self.enhancement_additional_probability = reader.read_f32_le().await?,
                19 => self.min_enhancement_possible_level = reader.read_f32_le().await?,
                20 => self.max_enhancement_possible_level = reader.read_f32_le().await?,
                21 => self.enhancement_target = Self::read_string(format, reader).await?,
                22 => self.extractable = reader.read_f32_le().await?,
                23 => self.taming_correction = reader.read_f32_le().await?,
                24 => self.usable_monster_level = reader.read_f32_le().await?,
                25 => self.attribute_type = Self::read_string(format, reader).await?,
                26 => self.creature_grade = Self::read_string(format, reader).await?,
                27 => self.color = Self::read_string(format, reader).await?,
                28 => self.durability = reader.read_f32_le().await?,
                29 => self.cooldown_share = reader.read_f32_le().await?,
                30 => self.cooldown = reader.read_f32_le().await?,
                31 => self.same_type_cooldown_no_share = reader.read_f32_le().await?,
                32 => self.purchase_price = reader.read_f32_le().await?,
                33 => self.disposal_price = reader.read_f32_le().await?,
                34 => self.stack_count = reader.read_f32_le().await?,
                35 => self.creation_count = reader.read_f32_le().await?,
                36 => self.scenario = Self::read_string(format, reader).await?,
                37 => self.area_type = Self::read_string(format, reader).await?,
                38 => self.usage_area = Self::read_string(format, reader).await?,
                39 => self.magic_stone_skill = Self::read_string(format, reader).await?,
                40 => self.magic_stone_skill_level = reader.read_f32_le().await?,
                41 => self.magic_stone_equip_effect_1 = Self::read_string(format, reader).await?,
                42 => self.magic_stone_equip_effect_2 = Self::read_string(format, reader).await?,
                43 => self.magic_stone_equip_effect_3 = Self::read_string(format, reader).await?,
                44 => self.magic_stone_equip_effect_4 = Self::read_string(format, reader).await?,
                45 => self.magic_stone_equip_effect_5 = Self::read_string(format, reader).await?,
                46 => {
                    self.skill_effect = {
                        let effect_skill = Self::read_string(format, reader).await?.to_uppercase();
                        if effect_skill != "*" {
                            Some(SharedString::new(effect_skill))
                        } else {
                            None
                        }
                    }
                }
                47 => self.no_drop = reader.read_f32_le().await?,
                48 => self.untradeable = reader.read_f32_le().await? != 0.0,
                49 => self.unsellable = reader.read_f32_le().await? != 0.0,
                50 => self.indestructible = reader.read_f32_le().await? != 0.0,
                51 => self.true_keeping = reader.read_f32_le().await?,
                52 => self.drop_level_check = Self::read_string(format, reader).await?,
                53 => self.binding = Binding::try_from(Self::read_string(format, reader).await?.as_str()).ok(),
                54 => self.binding_target = Self::read_string(format, reader).await?,
                55 => self.binding_release_count = reader.read_f32_le().await?,
                56 => self.consumption_limit = reader.read_f32_le().await?,
                57 => self.usage_restriction = Self::read_string(format, reader).await?,
                58 => self.sales_agent_category = Self::read_string(format, reader).await?,
                59 => self.creation_item = Self::read_string(format, reader).await?,
                60 => self.warp_map = Self::read_string(format, reader).await?,
                61 => self.warp_coordinates = Self::read_string(format, reader).await?,
                62 => self.combination_result = Self::read_string(format, reader).await?,
                63 => self.disappearance_time = reader.read_f32_le().await?,
                64 => self.seed_type = Self::read_string(format, reader).await?,
                65 => self.required_farm_level = reader.read_f32_le().await?,
                66 => self.crop_object = Self::read_string(format, reader).await?,
                67 => self.creation_fellow = Self::read_string(format, reader).await?,
                68 => self.usage_period = reader.read_f32_le().await?,
                69 => self.cash = reader.read_f32_le().await?,
                70 => self.fellow_experience = reader.read_f32_le().await?,
                71 => self.monster_summon = Self::read_string(format, reader).await?,
                72 => self.summon_time = reader.read_f32_le().await?,
                73 => self.drop_level_check_ignore = reader.read_f32_le().await?,
                74 => self.fellow_base_grade = Self::read_string(format, reader).await?,
                75 => self.gd_grade = Self::read_string(format, reader).await?,
                76 => self.raid_check = reader.read_f32_le().await?,
                77 => self.contents_level = reader.read_f32_le().await?,
                78 => self.engrave = reader.read_f32_le().await?,
                79 => self.character_level = reader.read_f32_le().await?,
                80 => self.purchase_limit = reader.read_f32_le().await?,
                81 => self.unified_channel_disabled = reader.read_f32_le().await?,
                82 => self.enhancement_possible_grade = Self::read_string(format, reader).await?,
                83 => self.enhancement_attempt_min_level = reader.read_f32_le().await?,
                84 => self.enhancement_attempt_max_level = reader.read_f32_le().await?,
                85 => self.disassembly_id = Self::read_string(format, reader).await?,
                86 => self.pvp_channel_usage = Self::read_string(format, reader).await?,
                87 => self.unusable_area = Self::read_string(format, reader).await?,
                88 => self.currency_settings_id = Self::read_string(format, reader).await?,
                _ => {}
            }
        }

        Ok(self)
    }
}


impl Item for Consume {
    fn set_locale(&mut self, locales: &HashMap<SharedString, Locale>, skill_locales: &HashMap<SharedString, Locale>) {
        self.locale = locales.get(&self.id).cloned();
        self.description_locale = locales.get(&SharedString::new(format!("{}_DESCRIPTION", self.id))).cloned();
    }

    async fn set_icon<R: std::io::Read + std::io::Seek>(
        &mut self,
        res: &HashMap<SharedString, super::item_res::ItemRes>,
        zip: &mut zip::ZipArchive<R>,
    ) -> Result<()> {
        if let Some(item_res) = res.get(&self.id) {
            if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon.to_lowercase())) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match Self::dds_to_jpeg(buf).await {
                    Ok(icon) => self.icon = Some(icon),
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            } else if let Ok(mut file) = zip.by_path(&format!(r"libs\ui\resources\textures\slot_icons\{}.dds", item_res.icon)) {
                let mut buf = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buf)?;

                match Self::dds_to_jpeg(buf).await {
                    Ok(icon) => self.icon = Some(icon),
                    Err(e) => warn!(?e, ?item_res, "Failed to load icon"),
                }
            }
        }
        Ok(())
    }

    fn get_full_type(&self) -> SharedString {
        unimplemented!()
    }

    fn get_type(&self) -> SharedString {
        unimplemented!()
    }

    fn set_item_set(&mut self, _item_set: &Vec<ItemSet>) {}

    fn set_product(
        &mut self,
        products_by_recipe_id: &HashMap<SharedString,  Rc<RefCell<Product>>>,
        products_by_result_id: &HashMap<SharedString,  Rc<RefCell<Product>>>,
    ) {
    }
}
