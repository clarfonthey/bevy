#[doc(no_inline)]
pub use crate::{
    app::prelude::*, core::prelude::*, ecs::prelude::*, hierarchy::prelude::*, input::prelude::*,
    log::prelude::*, math::prelude::*, reflect::prelude::*, time::prelude::*,
    transform::prelude::*, utils::prelude::*, window::prelude::*, DefaultPlugins, HeadlessPlugins,
    MinimalPlugins,
};

pub use bevy_derive::{bevy_main, Deref, DerefMut};

#[doc(no_inline)]
#[cfg(feature = "bevy_asset")]
pub use crate::asset::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_audio")]
pub use crate::audio::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_animation")]
pub use crate::animation::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_color")]
pub use crate::color::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_core_pipeline")]
pub use crate::core_pipeline::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_pbr")]
pub use crate::pbr::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_render")]
pub use crate::render::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_scene")]
pub use crate::scene::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_sprite")]
pub use crate::sprite::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_text")]
pub use crate::text::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_ui")]
pub use crate::ui::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_gizmos")]
pub use crate::gizmos::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_gilrs")]
pub use crate::gilrs::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_state")]
pub use crate::state::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_gltf")]
pub use crate::gltf::prelude::*;

#[doc(no_inline)]
#[cfg(feature = "bevy_picking")]
pub use crate::picking::prelude::*;
