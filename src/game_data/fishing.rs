use std::{collections::HashMap, io::SeekFrom};

use crate::game_data::{AsyncBufReadExtReadString, DataFormat, TagType, grade::Grade, items::ReadableItem, locale::Locale};
use anyhow::Result;
use gpui_kit::SharedString;
use indexmap::IndexMap;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

#[derive(Clone)]
pub struct FishingDrop {
    pub area_locale: Option<Locale>,
    pub map_locale: Option<Locale>,
    pub area: SharedString,
    pub map: SharedString,
    pub probability: f32,
    pub grade: Grade
}

#[derive(Default,Debug, Clone)]
pub struct Fishing {
    pub area_locale: Option<Locale>,
    pub map_locale: Option<Locale>,
    pub area: SharedString,
    pub map: SharedString,
    pub rodgroup: f32,
    pub groupno: Grade,
    pub mintime: f32,
    pub maxtime: f32,
    pub mnmintime: f32,
    pub mnmaxtime: f32,
    pub rankpoint: f32,
    pub probability: f32,
    pub autorate: f32,
    pub total: f32,
    pub total1: f32,
    pub rewards: IndexMap<u8, FishingReward>,
}

#[derive(Debug, Clone)]
pub struct FishingReward {
    pub id: SharedString,
    pub rate: f32,
    pub exp: f32,
    pub exp_rate: f32,
}

impl ReadableItem for Fishing {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        unimplemented!()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        unimplemented!()
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        let tag_count = definitions.len();
        let mut reward_ids: [SharedString; 20] = std::array::from_fn(|_| SharedString::default());
        let mut reward_rates = [0.0f32; 20];
        let mut reward_exps = [0.0f32; 20];
        let mut reward_exp_rates = [0.0f32; 20];
        for (tag_idx, (tag, _)) in definitions.iter().enumerate() {
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

            match tag.as_str() {
                "area" => self.area = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),
                "map" => self.map = SharedString::new(reader.read_string(Self::FORMAT).await?.to_uppercase()),
                "rodgroup" => self.rodgroup = reader.read_f32_le().await?,
                "groupno" => self.groupno = Grade::from(reader.read_f32_le().await? as u8 + 1),
                "mintime" => self.mintime = reader.read_f32_le().await?,
                "maxtime" => self.maxtime = reader.read_f32_le().await?,
                "mnmintime" => self.mnmintime = reader.read_f32_le().await?,
                "mnmaxtime" => self.mnmaxtime = reader.read_f32_le().await?,
                "rankpoint" => self.rankpoint = reader.read_f32_le().await?,
                "probability" => self.probability = reader.read_f32_le().await?,
                "autorate" => self.autorate = reader.read_f32_le().await?,

                "reward1" => reward_ids[0] =reader.read_string(Self::FORMAT).await?,
                "reward2" => reward_ids[1] = reader.read_string(Self::FORMAT).await?,
                "reward3" => reward_ids[2] = reader.read_string(Self::FORMAT).await?,
                "reward4" => reward_ids[3] = reader.read_string(Self::FORMAT).await?,
                "reward5" => reward_ids[4] = reader.read_string(Self::FORMAT).await?,
                "reward6" => reward_ids[5] = reader.read_string(Self::FORMAT).await?,
                "reward7" => reward_ids[6] =reader.read_string(Self::FORMAT).await?,
                "reward8" => reward_ids[7] = reader.read_string(Self::FORMAT).await?,
                "reward9" => reward_ids[8] = reader.read_string(Self::FORMAT).await?,
                "reward10" => reward_ids[9] =reader.read_string(Self::FORMAT).await?,
                "reward11" => reward_ids[10] = reader.read_string(Self::FORMAT).await?,
                "reward12" => reward_ids[11] = reader.read_string(Self::FORMAT).await?,
                "reward13" => reward_ids[12] = reader.read_string(Self::FORMAT).await?,
                "reward14" => reward_ids[13] = reader.read_string(Self::FORMAT).await?,
                "reward15" => reward_ids[14] = reader.read_string(Self::FORMAT).await?,
                "reward16" => reward_ids[15] = reader.read_string(Self::FORMAT).await?,
                "reward17" => reward_ids[16] = reader.read_string(Self::FORMAT).await?,
                "reward18" => reward_ids[17] = reader.read_string(Self::FORMAT).await?,
                "reward19" => reward_ids[18] = reader.read_string(Self::FORMAT).await?,
                "reward20" => reward_ids[19] = reader.read_string(Self::FORMAT).await?,

                "rewardrate1" => reward_rates[0] = reader.read_f32_le().await?,
                "rewardrate2" => reward_rates[1] = reader.read_f32_le().await?,
                "rewardrate3" => reward_rates[2] = reader.read_f32_le().await?,
                "rewardrate4" => reward_rates[3] = reader.read_f32_le().await?,
                "rewardrate5" => reward_rates[4] = reader.read_f32_le().await?,
                "rewardrate6" => reward_rates[5] = reader.read_f32_le().await?,
                "rewardrate7" => reward_rates[6] = reader.read_f32_le().await?,
                "rewardrate8" => reward_rates[7] = reader.read_f32_le().await?,
                "rewardrate9" => reward_rates[8] = reader.read_f32_le().await?,
                "rewardrate10" => reward_rates[9] = reader.read_f32_le().await?,
                "rewardrate11" => reward_rates[10] = reader.read_f32_le().await?,
                "rewardrate12" => reward_rates[11] = reader.read_f32_le().await?,
                "rewardrate13" => reward_rates[12] = reader.read_f32_le().await?,
                "rewardrate14" => reward_rates[13] = reader.read_f32_le().await?,
                "rewardrate15" => reward_rates[14] = reader.read_f32_le().await?,
                "rewardrate16" => reward_rates[15] = reader.read_f32_le().await?,
                "rewardrate17" => reward_rates[16] = reader.read_f32_le().await?,
                "rewardrate18" => reward_rates[17] = reader.read_f32_le().await?,
                "rewardrate19" => reward_rates[18] = reader.read_f32_le().await?,
                "rewardrate20" => reward_rates[19] = reader.read_f32_le().await?,

                "total" => self.total = reader.read_f32_le().await?,

                "rewardexp1" => reward_exps[0] = reader.read_f32_le().await?,
                "rewardexp2" => reward_exps[1] = reader.read_f32_le().await?,
                "rewardexp3" => reward_exps[2] = reader.read_f32_le().await?,
                "rewardexp4" => reward_exps[3] = reader.read_f32_le().await?,
                "rewardexp5" => reward_exps[4] = reader.read_f32_le().await?,
                "rewardexp6" => reward_exps[5] = reader.read_f32_le().await?,
                "rewardexp7" => reward_exps[6] = reader.read_f32_le().await?,
                "rewardexp8" => reward_exps[7] = reader.read_f32_le().await?,
                "rewardexp9" => reward_exps[8] = reader.read_f32_le().await?,
                "rewardexp10" => reward_exps[9] = reader.read_f32_le().await?,
                "rewardexp11" => reward_exps[10] = reader.read_f32_le().await?,
                "rewardexp12" => reward_exps[11] = reader.read_f32_le().await?,
                "rewardexp13" => reward_exps[12] = reader.read_f32_le().await?,
                "rewardexp14" => reward_exps[13] = reader.read_f32_le().await?,
                "rewardexp15" => reward_exps[14] = reader.read_f32_le().await?,
                "rewardexp16" => reward_exps[15] = reader.read_f32_le().await?,
                "rewardexp17" => reward_exps[16] = reader.read_f32_le().await?,
                "rewardexp18" => reward_exps[17] = reader.read_f32_le().await?,
                "rewardexp19" => reward_exps[18] = reader.read_f32_le().await?,
                "rewardexp20" => reward_exps[19] = reader.read_f32_le().await?,

                "rewardexprate1" => reward_exp_rates[0] = reader.read_f32_le().await?,
                "rewardexprate2" => reward_exp_rates[1] = reader.read_f32_le().await?,
                "rewardexprate3" => reward_exp_rates[2] = reader.read_f32_le().await?,
                "rewardexprate4" => reward_exp_rates[3] = reader.read_f32_le().await?,
                "rewardexprate5" => reward_exp_rates[4] = reader.read_f32_le().await?,
                "rewardexprate6" => reward_exp_rates[5] = reader.read_f32_le().await?,
                "rewardexprate7" => reward_exp_rates[6] = reader.read_f32_le().await?,
                "rewardexprate8" => reward_exp_rates[7] = reader.read_f32_le().await?,
                "rewardexprate9" => reward_exp_rates[8] = reader.read_f32_le().await?,
                "rewardexprate10" => reward_exp_rates[9] = reader.read_f32_le().await?,
                "rewardexprate11" => reward_exp_rates[10] = reader.read_f32_le().await?,
                "rewardexprate12" => reward_exp_rates[11] = reader.read_f32_le().await?,
                "rewardexprate13" => reward_exp_rates[12] = reader.read_f32_le().await?,
                "rewardexprate14" => reward_exp_rates[13] = reader.read_f32_le().await?,
                "rewardexprate15" => reward_exp_rates[14] = reader.read_f32_le().await?,
                "rewardexprate16" => reward_exp_rates[15] = reader.read_f32_le().await?,
                "rewardexprate17" => reward_exp_rates[16] = reader.read_f32_le().await?,
                "rewardexprate18" => reward_exp_rates[17] = reader.read_f32_le().await?,
                "rewardexprate19" => reward_exp_rates[18] = reader.read_f32_le().await?,
                "rewardexprate20" => reward_exp_rates[19] = reader.read_f32_le().await?,

                "total1" => self.total1 = reader.read_f32_le().await?,
                _ => {}
            }
        }
        for i in 0..20 {
            if reward_ids[i] != "*" && !reward_ids[i].is_empty() {
                self.rewards.insert(
                    i as u8,
                    FishingReward {
                        id: SharedString::new(reward_ids[i].to_uppercase()),
                        rate: reward_rates[i] * self.probability / 10_000_000_000.0,
                        exp: reward_exps[i],
                        exp_rate: reward_exp_rates[i],
                    },
                );
            }
        }
        Ok(self)
    }
}

impl Fishing {
    pub fn set_map_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.map_locale = locales.get(&self.map).cloned();
    }

    pub fn set_area_locale(&mut self, locales: &HashMap<SharedString, Locale>) {
        self.area_locale = locales.get(&self.area).cloned();
    }

        pub fn get_localized_fishing_area(&self) -> SharedString {
        self.area_locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.area.clone())
    }

    pub fn get_localized_fishing_map(&self) -> SharedString {
        self.map_locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.map.clone())
    }
}

impl FishingDrop {
    pub fn get_localized_fishing_area(&self) -> SharedString {
        self.area_locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.area.clone())
    }

    pub fn get_localized_fishing_map(&self) -> SharedString {
        self.map_locale.as_ref().and_then(|f| f.locale()).unwrap_or_else(|| self.map.clone())
    }
}
