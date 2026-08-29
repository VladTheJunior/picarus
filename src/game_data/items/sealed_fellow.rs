use std::io::SeekFrom;

use crate::game_data::{
    AsyncBufReadExtReadString, DataFormat, TagType,
    common::Common,
    effects::{ItemMinMaxNoStepEffect, ItemMinMaxStepEffect},
    item::ItemTrait,
    item::ReadableItem,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui::SharedString;

#[derive(Default)]
pub struct SealedFellow {
    pub debug: Vec<u8>,
    pub common: Common,

    pub effects: Vec<ItemMinMaxStepEffect>,
    pub max_enhancement_sealed_fellow_effect: Option<ItemMinMaxNoStepEffect>,
    pub tempering: u8,
    pub tempering_effect: f32,

    pub characteristic_power: u16,
}

impl ReadableItem for SealedFellow {
    const FORMAT: DataFormat = DataFormat::String;
    type Key = SharedString;

    fn key(&self) -> Self::Key {
        self.common.id.clone()
    }

    type CollectionItem = Self;
    fn new_collection_item(item: Self) -> Self::CollectionItem {
        item
    }

    fn debug_mut(&mut self) -> &mut Vec<u8> {
        &mut self.debug
    }

    async fn read<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(
        mut self,
        reader: &mut R,
        offsets: &[u32],
        item_idx: usize,
        definitions: &IndexMap<SharedString, TagType>,
        global_offset: u64,
    ) -> Result<Self> {
        self.parse_debug(reader, offsets, item_idx, definitions, global_offset).await?;
        let tag_count = definitions.len();
        for tag_idx in 0..tag_count {
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

            match tag_idx {
                0 => self.common.parse_id(reader, Self::FORMAT).await?,

                2 => self.common.parse_grade(reader).await?,

                5 => self.common.parse_item_level(reader).await?,

                9 => self.parse_effect(reader, Self::FORMAT).await?,
                10 => self.parse_effect(reader, Self::FORMAT).await?,
                11 => self.parse_effect(reader, Self::FORMAT).await?,
                12 => {
                    let effect = reader.read_string(Self::FORMAT).await?;
                    if effect != "*" && effect != "0" && !effect.starts_with("*,") && !effect.starts_with("0,") {
                        self.max_enhancement_sealed_fellow_effect = Some(ItemMinMaxNoStepEffect::new(&effect));
                    }
                }
                13 => self.tempering = reader.read_f32_le().await? as u8,
                14 => self.tempering_effect = reader.read_f32_le().await?,

                18 => self.common.parse_no_trade(reader).await?,
                19 => self.common.parse_no_sell(reader).await?,
                20 => self.common.parse_no_destroy(reader).await?,

                22 => self.common.parse_binding(reader, Self::FORMAT).await?,

                24 => self.characteristic_power = reader.read_f32_le().await? as u16,

                _ => {}
            }
        }

        Ok(self)
    }
}

impl SealedFellow {
    async fn parse_effect<R: AsyncBufReadExt + AsyncSeek + std::marker::Unpin>(&mut self, reader: &mut R, format: DataFormat) -> Result<()> {
        let effect = reader.read_string(format).await?;
        if effect != "*" && effect != "0" && !effect.starts_with("*,") && !effect.starts_with("0,") {
            self.effects.push(ItemMinMaxStepEffect::new(&effect));
        }

        Ok(())
    }
}

impl ItemTrait for SealedFellow {
    fn common(&self) -> &Common {
        &self.common
    }

    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn get_unique_effects(&self) -> std::collections::HashSet<SharedString> {
        let mut effects = std::collections::HashSet::new();
        effects.extend(self.effects.iter().filter_map(|f| f.parsed.as_ref().map(|(key, _, _, _)| key.clone())));

        if let Some(e) = self
            .max_enhancement_sealed_fellow_effect
            .as_ref()
            .and_then(|f| f.parsed.as_ref().map(|(key, _, _)| key))
        {
            effects.insert(e.clone());
        }
        effects
    }
}
