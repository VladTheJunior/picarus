use std::io::SeekFrom;

use crate::{
    game_data::{
        AsyncBufReadExtReadString, DataFormat, TagType,
        common::Common,
        effects::{EffectKind, ItemMinMaxNoStepEffect, ItemMinMaxStepEffect},
        items::{ItemTrait, ReadableItem},
    },
    game_data_view::PreviewBuilder,
};
use anyhow::Result;
use indexmap::IndexMap;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use gpui_kit::SharedString;

#[derive(Default, Clone)]
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
        for (tag_idx, (tag, tag_type)) in definitions.iter().enumerate() {
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
            self.common.parse(tag, tag_type, reader, Self::FORMAT).await?;
            match tag_idx {
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
        fn common_mut(&mut self) -> &mut Common {
        &mut self.common
    }
    fn debug(&self) -> &[u8] {
        &self.debug
    }

    fn get_unique_effects(&self) -> Vec<EffectKind> {
        let mut effects = Vec::new();
        effects.extend(self.effects.iter().map(|effect| EffectKind::MinMaxStep {
            id: self.common.id.clone(),
            effect: effect.clone(),
        }));

        if let Some(effect) = self.max_enhancement_sealed_fellow_effect.as_ref() {
            effects.push(EffectKind::MinMaxNoStep {
                id: self.common.id.clone(),
                effect: effect.clone(),
            });
        }
        effects
    }

    fn build_preview(&self) -> PreviewBuilder<'_> {
        PreviewBuilder::new(self.common())
            .fellow_stone_effects(
                self.effects.clone(),
                self.max_enhancement_sealed_fellow_effect.clone(),
                self.tempering,
                self.tempering_effect,
            )
            .talent_power(self.characteristic_power)
    }
}
