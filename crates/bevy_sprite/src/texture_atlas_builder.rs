use bevy_asset::AssetId;
use bevy_math::{URect, UVec2};
use bevy_render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::{Image, TextureFormatPixelInfo},
};
use bevy_utils::{
    tracing::{debug, error, warn},
    HashMap,
};
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, PackedLocation,
    RectToInsert, TargetBin,
};
use thiserror::Error;

use crate::{TextureAtlasLayout, TextureAtlasSettings, TextureAtlasSources};

#[derive(Debug, Error)]
pub enum TextureAtlasBuilderError {
    #[error("could not pack textures into an atlas within the given bounds")]
    NotEnoughSpace,
    #[error("added a texture with the wrong format in an atlas")]
    WrongFormat,
}

#[derive(Debug, Default)]
#[must_use]
/// A builder which is used to create a texture atlas from many individual
/// sprites.
pub struct TextureAtlasBuilder<'a> {
    /// Collection of texture's asset id (optional) and image data to be packed into an atlas
    textures_to_place: Vec<(Option<AssetId<Image>>, &'a Image)>,
    /// Settings for builder.
    settings: TextureAtlasSettings,
}

pub type TextureAtlasBuilderResult<T> = Result<T, TextureAtlasBuilderError>;

impl<'a> TextureAtlasBuilder<'a> {
    /// Sets the minimum size of the atlas in pixels.
    pub fn min_size(&mut self, size: UVec2) -> &mut Self {
        self.settings.min_size = size;
        self
    }

    /// Sets the maximum size of the atlas in pixels.
    pub fn max_size(&mut self, size: UVec2) -> &mut Self {
        self.settings.max_size = size;
        self
    }

    /// Sets the amount of margin in pixels to add around the entire atlas.
    ///
    /// This does not affect the padding between texture rects; that is [`padding`](Self::padding).
    pub fn margin(&mut self, margin: UVec2) -> &mut Self {
        self.settings.margin = margin;
        self
    }

    /// Sets the amount of padding in pixels to add between texture rects.
    ///
    /// This does not affect the margin between texture rects and the edge; that is [`margin`](Self::margin).
    pub fn padding(&mut self, padding: UVec2) -> &mut Self {
        self.settings.padding = padding;
        self
    }

    /// Force the atlas to convert all textures to the given format.
    ///
    /// If this setting is `None` (the default), then the builder will error
    /// unless all of the textures have the same format.
    pub fn convert_format(&mut self, format: Option<TextureFormat>) -> &mut Self {
        self.settings.convert_format = format;
        self
    }

    /// Sets all the settings for generating the texture atlas at once.
    pub fn settings(&mut self, settings: TextureAtlasSettings) -> &mut Self {
        self.settings = settings;
        self
    }

    /// Adds a texture to be copied to the texture atlas.
    ///
    /// Optionally an asset id can be passed that can later be used with the texture layout to retrieve the index of this texture.
    /// The insertion order will reflect the index of the added texture in the finished texture atlas.
    pub fn add_texture(
        &mut self,
        image_id: Option<AssetId<Image>>,
        texture: &'a Image,
    ) -> &mut Self {
        self.textures_to_place.push((image_id, texture));
        self
    }

    fn copy_texture_to_atlas(
        atlas_texture: &mut Image,
        texture: &Image,
        packed_location: &PackedLocation,
        padding: UVec2,
    ) {
        let rect_width = (packed_location.width() - padding.x) as usize;
        let rect_height = (packed_location.height() - padding.y) as usize;
        let rect_x = packed_location.x() as usize;
        let rect_y = packed_location.y() as usize;
        let atlas_width = atlas_texture.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size();

        for (texture_y, bound_y) in (rect_y..rect_y + rect_height).enumerate() {
            let begin = (bound_y * atlas_width + rect_x) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_texture.data[begin..end]
                .copy_from_slice(&texture.data[texture_begin..texture_end]);
        }
    }

    fn copy_converted_texture(
        &self,
        atlas_texture: &mut Image,
        texture: &Image,
        packed_location: &PackedLocation,
        convert_format: TextureFormat,
    ) {
        if convert_format == texture.texture_descriptor.format {
            Self::copy_texture_to_atlas(
                atlas_texture,
                texture,
                packed_location,
                self.settings.padding,
            );
        } else if let Some(converted_texture) = texture.convert(convert_format) {
            debug!(
                "Converting texture from '{:?}' to '{:?}'",
                texture.texture_descriptor.format, convert_format
            );
            Self::copy_texture_to_atlas(
                atlas_texture,
                &converted_texture,
                packed_location,
                self.settings.padding,
            );
        } else {
            error!(
                "Error converting texture from '{:?}' to '{:?}', ignoring",
                texture.texture_descriptor.format, convert_format
            );
        }
    }

    #[deprecated(
        since = "0.14.0",
        note = "TextureAtlasBuilder::finish() was not idiomatic. Use TextureAtlasBuilder::build() instead."
    )]
    pub fn finish(
        &mut self,
    ) -> Result<(TextureAtlasLayout, TextureAtlasSources, Image), TextureAtlasBuilderError> {
        self.build()
    }

    /// Consumes the builder, and returns the newly created texture atlas and
    /// the associated atlas layout.
    ///
    /// Assigns indices to the textures based on the insertion order.
    /// Internally it copies all rectangles from the textures and copies them
    /// into a new texture.
    ///
    /// # Usage
    ///
    /// ```rust
    /// # use bevy_sprite::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::*;
    /// # use bevy_render::prelude::*;
    ///
    /// fn my_system(mut commands: Commands, mut textures: ResMut<Assets<Image>>, mut layouts: ResMut<Assets<TextureAtlasLayout>>) {
    ///     // Declare your builder
    ///     let mut builder = TextureAtlasBuilder::default();
    ///     // Customize it
    ///     // ...
    ///     // Build your texture and the atlas layout
    ///     let (atlas_layout, atlas_sources, texture) = builder.build().unwrap();
    ///     let texture = textures.add(texture);
    ///     let layout = layouts.add(atlas_layout);
    ///     // Spawn your sprite
    ///     commands.spawn((
    ///         SpriteBundle { texture, ..Default::default() },
    ///         TextureAtlas::from(layout),
    ///     ));
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// If there is not enough space in the atlas texture, an error will
    /// be returned. It is then recommended to make a larger sprite sheet.
    pub fn build(
        &mut self,
    ) -> Result<(TextureAtlasLayout, TextureAtlasSources, Image), TextureAtlasBuilderError> {
        // extra padding on bottom-right of atlas gets trimmed,
        // but extra margin gets added on all four sides
        let max_size = (self.settings.max_size + self.settings.padding)
            .saturating_sub(2 * self.settings.margin);
        let max_width = max_size.x;
        let max_height = max_size.y;

        let mut current_width = self.settings.min_size.x;
        let mut current_height = self.settings.min_size.y;
        let mut rect_placements = None;
        let mut atlas_texture = Image::default();
        let mut rects_to_place = GroupedRectsToPlace::<usize>::new();

        // get unified texture format
        let unified_format = match self.settings.convert_format {
            Some(format) => format,
            None => match self.textures_to_place.split_first() {
                Some(((_, image), rest)) => {
                    let format = image.texture_descriptor.format;
                    for (_, image) in rest {
                        if image.texture_descriptor.format != format {
                            warn!(
                                "Loading textures of different formats '{:?}' and '{:?}' without a conversion format specified",
                                image.texture_descriptor.format, format
                            );
                            return Err(TextureAtlasBuilderError::WrongFormat);
                        }
                    }
                    format
                }
                None => {
                    warn!("Creating an atlas of no textures without a conversion format specified");
                    return Err(TextureAtlasBuilderError::WrongFormat);
                }
            },
        };

        // Adds textures to rectangle group packer
        for (index, (_, texture)) in self.textures_to_place.iter().enumerate() {
            rects_to_place.push_rect(
                index,
                None,
                RectToInsert::new(
                    texture.width() + self.settings.padding.x,
                    texture.height() + self.settings.padding.y,
                    1,
                ),
            );
        }

        while rect_placements.is_none() {
            if current_width > max_width || current_height > max_height {
                break;
            }

            let last_attempt = current_height == max_height && current_width == max_width;

            let mut target_bins = std::collections::BTreeMap::new();
            target_bins.insert(0, TargetBin::new(current_width, current_height, 1));
            rect_placements = match pack_rects(
                &rects_to_place,
                &mut target_bins,
                &volume_heuristic,
                &contains_smallest_box,
            ) {
                Ok(rect_placements) => {
                    // if there were any rects placed, there is extra padding on them;
                    // remove this, but don't go below minimum width
                    if !self.textures_to_place.is_empty() {
                        current_width = current_width
                            .saturating_sub(self.settings.padding.x)
                            .max(self.settings.min_size.x);
                        current_height = current_height
                            .saturating_sub(self.settings.padding.x)
                            .max(self.settings.min_size.y);
                    }

                    // add margin, which is on both sides
                    current_width += 2 * self.settings.margin.x;
                    current_height += 2 * self.settings.margin.x;
                    atlas_texture = Image::new(
                        Extent3d {
                            width: current_width,
                            height: current_height,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        vec![
                            0;
                            unified_format.pixel_size() * (current_width * current_height) as usize
                        ],
                        unified_format,
                        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                    );
                    Some(rect_placements)
                }
                Err(rectangle_pack::RectanglePackError::NotEnoughBinSpace) => {
                    current_height = (current_height * 2).min(max_height);
                    current_width = (current_width * 2).min(max_width);
                    None
                }
            };

            if last_attempt {
                break;
            }
        }

        let rect_placements = rect_placements.ok_or(TextureAtlasBuilderError::NotEnoughSpace)?;

        let mut texture_rects = Vec::with_capacity(rect_placements.packed_locations().len());
        let mut texture_ids = HashMap::default();
        // We iterate through the textures to place to respect the insertion order for the texture indices
        for (index, (image_id, texture)) in self.textures_to_place.iter().enumerate() {
            let (_, packed_location) = rect_placements.packed_locations().get(&index).unwrap();

            let min = self.settings.margin + UVec2::new(packed_location.x(), packed_location.y());
            let max = min + UVec2::new(packed_location.width(), packed_location.height())
                - self.settings.padding;
            if let Some(image_id) = image_id {
                texture_ids.insert(*image_id, index);
            }
            texture_rects.push(URect { min, max });
            self.copy_converted_texture(
                &mut atlas_texture,
                texture,
                packed_location,
                unified_format,
            );
        }

        Ok((
            TextureAtlasLayout {
                size: atlas_texture.size(),
                textures: texture_rects,
            },
            TextureAtlasSources { texture_ids },
            atlas_texture,
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::{TextureAtlasBuilder, TextureAtlasLayout, TextureAtlasSettings};
    use bevy_math::{URect, UVec2};
    use bevy_render::{render_resource::TextureFormat, texture::Image};

    #[test]
    fn trivial_texture_atlas() {
        let format = TextureFormat::Rgba8UnormSrgb;

        // be a bit sneaky:
        //
        // * ensure margin is added onto min size
        // * ensure padding doesn't affect anything
        let settings = TextureAtlasSettings {
            min_size: UVec2::new(256, 256),
            max_size: UVec2::new(384, 384),
            padding: UVec2::new(1024, 1024),
            margin: UVec2::new(64, 64),
            convert_format: Some(format),
        };

        // this should work whether we have a single image or not, as long as the
        // image fits within the min size
        for texture in [None, Some(&Image::transparent())] {
            let mut builder = TextureAtlasBuilder::default();
            builder.settings(settings);
            if let Some(texture) = texture {
                builder.add_texture(None, texture);
            }
            let (layout, sources, image) = builder.build().unwrap();
            let mut textures = Vec::new();
            if texture.is_some() {
                textures.push(URect::new(
                    settings.margin.x,
                    settings.margin.y,
                    settings.margin.x + 1,
                    settings.margin.y + 1,
                ));
            }
            assert_eq!(
                layout,
                TextureAtlasLayout {
                    size: settings.max_size,
                    textures,
                }
            );
            assert!(sources.texture_ids.is_empty());
            assert_eq!(image.size(), settings.max_size);
        }
    }

    #[test]
    fn nonempty_texture_atlas() {
        let settings = TextureAtlasSettings {
            min_size: UVec2::new(256, 256),
            max_size: UVec2::new(1154, 1154),
            padding: UVec2::new(1024, 1024),
            margin: UVec2::new(64, 64),
            convert_format: Some(TextureFormat::Rgba8UnormSrgb),
        };

        let (layout, sources, image) = TextureAtlasBuilder::default()
            .settings(settings)
            .add_texture(None, &Image::default())
            .add_texture(None, &Image::default())
            .build()
            .unwrap();
        assert_eq!(
            layout,
            TextureAtlasLayout {
                size: settings.max_size,
                textures: vec![
                    URect::new(
                        settings.margin.x,
                        settings.margin.y,
                        settings.margin.x + 1,
                        settings.margin.y + 1
                    ),
                    URect::new(
                        settings.margin.x + 1 + settings.padding.x,
                        settings.margin.y,
                        settings.margin.x + 2 + settings.padding.x,
                        settings.margin.y + 1,
                    ),
                ],
            }
        );
        assert!(sources.texture_ids.is_empty());
        assert_eq!(image.size(), settings.max_size);
    }
}
