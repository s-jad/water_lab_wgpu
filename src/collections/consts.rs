pub(crate) const SCREEN_WIDTH: u32 = 1376;
pub(crate) const SCREEN_HEIGHT: u32 = 768;
pub(crate) const ASPECT: f32 = SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32;

pub(crate) const TERRAIN_TEXTURE_WIDTH: u32 = 2048;
pub(crate) const TERRAIN_TEXTURE_HEIGHT: u32 = 2048;

pub(crate) const TERRAIN_TEX_DISPATCH_SIZE_X: u32 =
    ((TERRAIN_TEXTURE_WIDTH).saturating_add(32)) / 32;
pub(crate) const TERRAIN_TEX_DISPATCH_SIZE_Y: u32 =
    ((TERRAIN_TEXTURE_HEIGHT).saturating_add(32)) / 32;

pub(crate) const TERRAIN_TEX_BUF_SIZE: usize = TERRAIN_TEXTURE_WIDTH as usize
    * TERRAIN_TEXTURE_HEIGHT as usize
    * 4
    * (std::mem::size_of::<f32>());
